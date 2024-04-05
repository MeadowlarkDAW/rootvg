use rootvg_core::color::RGBA8;
use rootvg_core::math::{Point, Size};

use super::RcTextBuffer;

#[derive(Debug, Clone, PartialEq)]
pub struct TextPrimitive {
    pub buffer: RcTextBuffer,
    pub pos: Point,
    pub color: RGBA8,
    pub bounds_size: Size,
}

impl TextPrimitive {
    pub fn new(buffer: RcTextBuffer, pos: Point, color: RGBA8) -> Self {
        let bounds_size = buffer.bounds_size();

        Self {
            buffer,
            pos,
            color,
            bounds_size,
        }
    }
}
