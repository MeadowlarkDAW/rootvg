// The following code was copied and modified from
// https://github.com/iced-rs/iced/blob/31d1d5fecbef50fa319cabd5d4194f1e4aaefa21/core/src/shadow.rs
// Iced license (MIT): https://github.com/iced-rs/iced/blob/31d1d5fecbef50fa319cabd5d4194f1e4aaefa21/LICENSE

use rootvg_core::color::PackedSrgb;
use rootvg_core::math::Point;

/// A shadow.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Shadow {
    /// The color of the shadow.
    pub color: PackedSrgb,

    /// The offset of the shadow in logical points.
    pub offset: Point,

    /// The blur radius of the shadow in logical points.
    pub blur_radius: f32,
}
