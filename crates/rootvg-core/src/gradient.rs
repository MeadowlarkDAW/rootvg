// The following code was copied and modified from
// https://github.com/iced-rs/iced/blob/31d1d5fecbef50fa319cabd5d4194f1e4aaefa21/graphics/src/gradient.rs
// Iced license (MIT): https://github.com/iced-rs/iced/blob/31d1d5fecbef50fa319cabd5d4194f1e4aaefa21/LICENSE

use half::f16;
use std::cmp::Ordering;
use std::f32::consts::FRAC_PI_2;

use super::color::PackedSrgb;
use crate::math::{Angle, Point, Rect};

pub const MAX_STOPS: usize = 8;

/// A fill which transitions colors progressively along a direction, either linearly, radially (TBD),
/// or conically (TBD).
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Gradient {
    /// A linear gradient interpolates colors along a direction at a specific angle.
    Linear(LinearGradient),
}

impl Gradient {
    /// Adjust the opacity of the gradient by a multiplier applied to each color stop.
    pub fn mul_alpha(mut self, alpha_multiplier: f32) -> Self {
        match &mut self {
            Gradient::Linear(linear) => {
                for stop in linear.stops.iter_mut().flatten() {
                    *stop.color.a_mut() *= alpha_multiplier;
                }
            }
        }

        self
    }

    pub fn packed(&self, bounds: Rect) -> PackedGradient {
        PackedGradient::new(self, bounds)
    }
}

impl From<LinearGradient> for Gradient {
    fn from(gradient: LinearGradient) -> Self {
        Self::Linear(gradient)
    }
}

impl Default for Gradient {
    fn default() -> Self {
        Gradient::Linear(LinearGradient::new(Angle::default()))
    }
}

/// A point along the gradient vector where the specified [`color`] is unmixed.
///
/// [`color`]: Self::color
#[derive(Debug, Default, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ColorStop {
    /// Offset along the gradient vector in the range `[0.0, 1.0]`.
    pub offset: f32,

    /// The color of the gradient at the specified [`offset`].
    ///
    /// [`offset`]: Self::offset
    pub color: PackedSrgb,
}

/// A linear gradient.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LinearGradient {
    /// How the [`Gradient`] is angled within its bounds.
    pub angle: Angle,
    /// [`ColorStop`]s along the linear gradient path.
    pub stops: [Option<ColorStop>; MAX_STOPS],
}

impl LinearGradient {
    /// Creates a new [`Linear`] gradient with the given angle in [`Angle`].
    pub const fn new(angle: Angle) -> Self {
        Self {
            angle,
            stops: [None; 8],
        }
    }

    /// Adds a new [`ColorStop`], defined by an offset and a color, to the gradient.
    ///
    /// Any `offset` that is not within `0.0..=1.0` will be silently ignored.
    ///
    /// Any stop added after the 8th will be silently ignored.
    pub fn add_stop(mut self, offset: f32, color: impl Into<PackedSrgb>) -> Self {
        if offset.is_finite() && (0.0..=1.0).contains(&offset) {
            let (Ok(index) | Err(index)) = self.stops.binary_search_by(|stop| match stop {
                None => Ordering::Greater,
                Some(stop) => stop.offset.partial_cmp(&offset).unwrap(),
            });

            if index < 8 {
                self.stops[index] = Some(ColorStop {
                    offset,
                    color: color.into(),
                });
            }
        } else {
            log::warn!("Gradient color stop must be within 0.0..=1.0 range.");
        };

        self
    }

    /// Adds multiple [`ColorStop`]s to the gradient.
    ///
    /// Any stop added after the 8th will be silently ignored.
    pub fn add_stops(mut self, stops: impl IntoIterator<Item = ColorStop>) -> Self {
        for stop in stops {
            self = self.add_stop(stop.offset, stop.color);
        }

        self
    }
}

/// Packed [`Gradient`] data for use in shader code.
#[repr(C)]
#[derive(Default, Debug, Copy, Clone, PartialEq, bytemuck::Zeroable, bytemuck::Pod)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PackedGradient {
    /// 8 colors, each channel = 16 bit float, 2 colors packed into 1 u32
    pub colors: [[u32; 2]; 8],
    /// 8 offsets, 8x 16 bit floats packed into 4 u32s
    pub offsets: [u32; 4],
    /// `[start.x, start.y, end.x, end.y]` in logical points
    pub direction: [f32; 4],
}

impl PackedGradient {
    pub fn new(gradient: &Gradient, bounds: Rect) -> Self {
        match gradient {
            Gradient::Linear(linear) => {
                let mut colors = [[0u32; 2]; 8];
                let mut offsets = [f16::from(0u8); 8];

                for (index, stop) in linear.stops.iter().enumerate() {
                    let packed_color = stop.map(|s| s.color).unwrap_or(PackedSrgb::default());

                    colors[index] = [
                        pack_f16s([
                            f16::from_f32(packed_color.r()),
                            f16::from_f32(packed_color.g()),
                        ]),
                        pack_f16s([
                            f16::from_f32(packed_color.b()),
                            f16::from_f32(packed_color.a()),
                        ]),
                    ];

                    offsets[index] = f16::from_f32(stop.map(|s| s.offset).unwrap_or(2.0));
                }

                let offsets = [
                    pack_f16s([offsets[0], offsets[1]]),
                    pack_f16s([offsets[2], offsets[3]]),
                    pack_f16s([offsets[4], offsets[5]]),
                    pack_f16s([offsets[6], offsets[7]]),
                ];

                let (start, end) = to_distance(linear.angle, &bounds);

                let direction = [start.x, start.y, end.x, end.y];

                PackedGradient {
                    colors,
                    offsets,
                    direction,
                }
            }
        }
    }
}

/// Calculates the line in which the angle intercepts the `bounds`.
fn to_distance(angle: Angle, bounds: &Rect) -> (Point, Point) {
    let angle = angle - Angle { radians: FRAC_PI_2 };

    let r = Point::new(f32::cos(angle.radians), f32::sin(angle.radians));
    let bounds_center = bounds.center();

    let distance_to_rect = f32::max(
        f32::abs(r.x * bounds.size.width / 2.0),
        f32::abs(r.y * bounds.size.height / 2.0),
    );

    let start = Point::new(
        bounds_center.x - (r.x * distance_to_rect),
        bounds_center.y - (r.y * distance_to_rect),
    );
    let end = Point::new(
        bounds_center.x + (r.x * distance_to_rect),
        bounds_center.y + (r.y * distance_to_rect),
    );

    (start, end)
}

/// Packs two f16s into one u32.
fn pack_f16s(f: [f16; 2]) -> u32 {
    let one = (f[0].to_bits() as u32) << 16;
    let two = f[1].to_bits() as u32;

    one | two
}
