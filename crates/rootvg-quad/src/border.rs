// The following code was copied and modified from
// https://github.com/iced-rs/iced/blob/31d1d5fecbef50fa319cabd5d4194f1e4aaefa21/core/src/border.rs
// Iced license (MIT): https://github.com/iced-rs/iced/blob/31d1d5fecbef50fa319cabd5d4194f1e4aaefa21/LICENSE

use rootvg_core::color::PackedSrgb;

/// A struct defining a border.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Radius {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl Radius {
    pub const fn new(top_left: f32, top_right: f32, bottom_right: f32, bottom_left: f32) -> Self {
        Self {
            top_left,
            top_right,
            bottom_right,
            bottom_left,
        }
    }

    pub const fn all_same(val: f32) -> Self {
        Self {
            top_left: val,
            top_right: val,
            bottom_right: val,
            bottom_left: val,
        }
    }

    pub const ZERO: Self = Self {
        top_left: 0.0,
        top_right: 0.0,
        bottom_right: 0.0,
        bottom_left: 0.0,
    };

    pub const CIRCLE: Self = Self {
        top_left: 1000000.0,
        top_right: 1000000.0,
        bottom_right: 1000000.0,
        bottom_left: 1000000.0,
    };
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

/// An alias for `Radius::new(val, val, val, val)`
pub const fn radius(val: f32) -> Radius {
    Radius::all_same(val)
}
