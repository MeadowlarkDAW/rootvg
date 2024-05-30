use rootvg_core::color::RGBA8;
use rootvg_core::math::{Point, Rect};

use super::RcTextBuffer;

#[derive(Debug, Clone, PartialEq)]
pub struct TextPrimitive {
    pub buffer: RcTextBuffer,
    pub pos: Point,
    pub color: RGBA8,
    pub clipping_bounds: Rect,
}

impl TextPrimitive {
    /// Create a new [`TextPrimitive`]
    ///
    /// * `buffer` - The text buffer
    /// * `pos` - The position of the primitive
    /// * `color` - The color of the text
    /// * `clipping_bounds` - A clipping rectangle to apply to the text (relative to the text buffer).
    /// If this is set to `None`, then a default clipping rectangle covering the whole text buffer
    /// will be used.
    pub fn new(
        buffer: RcTextBuffer,
        pos: Point,
        color: RGBA8,
        clipping_bounds: Option<Rect>,
    ) -> Self {
        let clipping_bounds =
            clipping_bounds.unwrap_or_else(|| Rect::from_size(buffer.bounds_size()));
        Self {
            buffer,
            pos,
            color,
            clipping_bounds,
        }
    }
}
