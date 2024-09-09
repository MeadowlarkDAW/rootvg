use euclid::default::{Size2D, Transform2D};

use crate::Color;

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct Paint {
    pub transform: Option<Transform2D<f32>>,
    pub paint_type: PaintType,
}

impl Paint {
    pub const fn new(paint_type: PaintType) -> Self {
        Self {
            transform: None,
            paint_type,
        }
    }

    pub fn solid_color(color: impl Into<Color>) -> Self {
        Self {
            transform: None,
            paint_type: PaintType::SolidColor(color.into()),
        }
    }

    pub fn gradient(
        inner_color: impl Into<Color>,
        outer_color: impl Into<Color>,
        extent: Option<Size2D<f32>>,
        radius: f32,
        feather: f32,
    ) -> Self {
        Self {
            transform: None,
            paint_type: PaintType::Gradient {
                inner_color: inner_color.into(),
                outer_color: outer_color.into(),
                extent,
                radius,
                feather,
            },
        }
    }

    pub fn image(image_id: u32) -> Self {
        Self {
            transform: None,
            paint_type: PaintType::Image {
                image_id,
                extent: None,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PaintType {
    SolidColor(Color),
    Gradient {
        inner_color: Color,
        outer_color: Color,
        extent: Option<Size2D<f32>>,
        radius: f32,
        feather: f32,
    },
    Image {
        image_id: u32,
        extent: Option<Size2D<f32>>,
    },
}

impl Default for PaintType {
    fn default() -> Self {
        Self::SolidColor(crate::color::BLACK)
    }
}

impl From<Color> for PaintType {
    fn from(c: Color) -> Self {
        Self::SolidColor(c)
    }
}

impl From<Color> for Paint {
    fn from(c: Color) -> Self {
        Self::solid_color(c)
    }
}
