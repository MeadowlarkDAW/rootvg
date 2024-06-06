// The following code was copied and modified from
// https://github.com/iced-rs/iced/blob/31d1d5fecbef50fa319cabd5d4194f1e4aaefa21/core/src/border.rs
// Iced license (MIT): https://github.com/iced-rs/iced/blob/31d1d5fecbef50fa319cabd5d4194f1e4aaefa21/LICENSE

use rootvg_core::color::PackedSrgb;

/// A struct defining a border.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Border {
    /// The color of the border.
    pub color: PackedSrgb,

    /// The width of the border in logical points.
    pub width: f32,

    /// The radius of the border in logical points.
    pub radius: Radius,
}

/// The border radii in logical points
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Radius {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl Radius {
    pub const fn zero() -> Self {
        Self {
            top_left: 0.0,
            top_right: 0.0,
            bottom_right: 0.0,
            bottom_left: 0.0,
        }
    }
}

impl From<f32> for Radius {
    fn from(w: f32) -> Self {
        Self {
            top_left: w,
            top_right: w,
            bottom_right: w,
            bottom_left: w,
        }
    }
}

impl From<u8> for Radius {
    fn from(w: u8) -> Self {
        Self {
            top_left: f32::from(w),
            top_right: f32::from(w),
            bottom_right: f32::from(w),
            bottom_left: f32::from(w),
        }
    }
}

impl From<[f32; 4]> for Radius {
    fn from(radi: [f32; 4]) -> Self {
        Self {
            top_left: radi[0],
            top_right: radi[1],
            bottom_right: radi[2],
            bottom_left: radi[3],
        }
    }
}

impl From<Radius> for [f32; 4] {
    fn from(radi: Radius) -> Self {
        [
            radi.top_left,
            radi.top_right,
            radi.bottom_right,
            radi.bottom_left,
        ]
    }
}
