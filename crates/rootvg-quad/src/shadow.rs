// The following code was copied and modified from
// https://github.com/iced-rs/iced/blob/31d1d5fecbef50fa319cabd5d4194f1e4aaefa21/core/src/shadow.rs
// Iced license (MIT): https://github.com/iced-rs/iced/blob/31d1d5fecbef50fa319cabd5d4194f1e4aaefa21/LICENSE

use rootvg_core::color::PackedSrgb;
use rootvg_core::math::Vector;

/// A shadow.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Shadow {
    /// The color of the shadow.
    pub color: PackedSrgb,

    /// The offset of the shadow in logical points.
    pub offset: Vector,

    /// The blur radius of the shadow in logical points.
    pub blur_radius: f32,
}
