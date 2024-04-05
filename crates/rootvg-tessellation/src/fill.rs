// The following code was copied and modified from
// https://github.com/iced-rs/iced/blob/31d1d5fecbef50fa319cabd5d4194f1e4aaefa21/graphics/src/geometry/fill.rs
// Iced license (MIT): https://github.com/iced-rs/iced/blob/31d1d5fecbef50fa319cabd5d4194f1e4aaefa21/LICENSE

use rootvg_core::color::{PackedSrgb, RGB8, RGBA8};

#[cfg(feature = "gradient")]
use rootvg_core::gradient::PackedGradient;

/// The style used to fill geometry.
#[derive(Debug, Clone)]
pub struct Fill {
    /// The color or gradient of the fill.
    ///
    /// By default, it is set to [`FillStyle::Solid`] with [`Color::BLACK`].
    pub style: FillStyle,

    /// The fill rule defines how to determine what is inside and what is
    /// outside of a shape.
    ///
    /// See the [SVG specification][1] for more details.
    ///
    /// By default, it is set to `NonZero`.
    ///
    /// [1]: https://www.w3.org/TR/SVG/painting.html#FillRuleProperty
    pub rule: FillRule,
}

impl Default for Fill {
    fn default() -> Self {
        Self {
            style: FillStyle::Solid(PackedSrgb([0.0, 0.0, 0.0, 1.0])),
            rule: FillRule::NonZero,
        }
    }
}

impl From<PackedSrgb> for Fill {
    fn from(color: PackedSrgb) -> Fill {
        Fill {
            style: FillStyle::Solid(color),
            ..Fill::default()
        }
    }
}

impl From<RGB8> for Fill {
    fn from(color: RGB8) -> Fill {
        Fill {
            style: FillStyle::Solid(color.into()),
            ..Fill::default()
        }
    }
}

impl From<RGBA8> for Fill {
    fn from(color: RGBA8) -> Fill {
        Fill {
            style: FillStyle::Solid(color.into()),
            ..Fill::default()
        }
    }
}

#[cfg(feature = "gradient")]
impl From<PackedGradient> for Fill {
    fn from(gradient: PackedGradient) -> Self {
        Fill {
            style: FillStyle::Gradient(gradient),
            ..Default::default()
        }
    }
}

/// The fill rule defines how to determine what is inside and what is outside of
/// a shape.
///
/// See the [SVG specification][1].
///
/// [1]: https://www.w3.org/TR/SVG/painting.html#FillRuleProperty
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillRule {
    #[default]
    NonZero,
    EvenOdd,
}

/// The coloring style of some drawing.
#[derive(Debug, Clone, PartialEq)]
pub enum FillStyle {
    /// A solid [`Color`].
    Solid(PackedSrgb),

    #[cfg(feature = "gradient")]
    /// A [`PackedGradient`] color.
    Gradient(PackedGradient),
}

impl From<PackedSrgb> for FillStyle {
    fn from(color: PackedSrgb) -> Self {
        Self::Solid(color)
    }
}

#[cfg(feature = "gradient")]
impl From<PackedGradient> for FillStyle {
    fn from(gradient: PackedGradient) -> Self {
        Self::Gradient(gradient)
    }
}

impl From<RGB8> for FillStyle {
    fn from(color: RGB8) -> Self {
        Self::Solid(color.into())
    }
}

impl From<RGBA8> for FillStyle {
    fn from(color: RGBA8) -> Self {
        Self::Solid(color.into())
    }
}
