use rootvg_core::color::PackedSrgb;
use rootvg_core::gradient::{Gradient, PackedGradient};
use rootvg_core::math::{Point, Rect, Size};

use crate::border::Border;
use crate::Radius;

/// A quad primitive with a gradient background.
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct GradientQuad {
    /// The bounds of the quad in logical points.
    pub bounds: Rect,
    /// The background color of the quad
    pub bg_gradient: Gradient,
    /// The [`Border`] of the quad
    pub border: Border,
    /*
    /// The shadow of the quad
    pub shadow: Shadow,
    */
}

impl GradientQuad {
    pub fn packed(&self) -> GradientQuadPrimitive {
        GradientQuadPrimitive {
            gradient: self.bg_gradient.packed(self.bounds),
            position: self.bounds.origin.into(),
            size: self.bounds.size.into(),
            border_color: self.border.color,
            border_radius: self.border.radius.into(),
            border_width: self.border.width,
        }
    }

    pub fn builder(size: Size) -> GradientQuadBuilder {
        GradientQuadBuilder::new(size)
    }
}

pub struct GradientQuadBuilder {
    quad: GradientQuad,
}

impl GradientQuadBuilder {
    pub fn new(size: Size) -> Self {
        Self {
            quad: GradientQuad {
                bounds: Rect {
                    origin: Point::new(0.0, 0.0),
                    size,
                },
                ..Default::default()
            },
        }
    }

    pub fn position(mut self, position: Point) -> Self {
        self.quad.bounds.origin = position;
        self
    }

    pub fn bg_gradient(mut self, color: impl Into<Gradient>) -> Self {
        self.quad.bg_gradient = color.into();
        self
    }

    pub fn border_color(mut self, color: impl Into<PackedSrgb>) -> Self {
        self.quad.border.color = color.into();
        self
    }

    pub fn border_width(mut self, width: f32) -> Self {
        self.quad.border.width = width;
        self
    }

    pub fn border_radius(mut self, radius: impl Into<Radius>) -> Self {
        self.quad.border.radius = radius.into();
        self
    }

    pub fn border(mut self, border: Border) -> Self {
        self.quad.border = border;
        self
    }

    /*
    pub fn shadow_color(mut self, color: impl Into<PackedSrgb>) -> Self {
        self.quad.shadow.color = color.into();
        self
    }

    pub fn shadow_offset(mut self, offset: Vector) -> Self {
        self.quad.shadow.offset = offset;
        self
    }

    pub fn blur_radius(mut self, blur_radius: f32) -> Self {
        self.quad.shadow.blur_radius = blur_radius;
        self
    }

    pub fn shadow(mut self, shadow: Shadow) -> Self {
        self.quad.shadow = shadow;
        self
    }
    */

    pub fn build(self) -> GradientQuad {
        self.quad
    }
}

/// A quad primitive with a gradient background, packed into a format
/// for use in rendering.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Zeroable, bytemuck::Pod)]
pub struct GradientQuadPrimitive {
    /// The background gradient data of the quad.
    pub gradient: PackedGradient,

    /// The position of the [`Quad`] in logical points.
    pub position: [f32; 2],

    /// The size of the [`Quad`] in logical points.
    pub size: [f32; 2],

    /// The border color of the [`Quad`], in __linear RGB__.
    pub border_color: PackedSrgb,

    /// The border radii of the [`Quad`] in logical points.
    pub border_radius: [f32; 4],

    /// The border width of the [`Quad`] in logical points.
    pub border_width: f32,
}

impl GradientQuadPrimitive {
    pub fn new(quad: &GradientQuad) -> Self {
        Self {
            gradient: quad.bg_gradient.packed(quad.bounds),
            position: quad.bounds.origin.into(),
            size: quad.bounds.size.into(),
            border_color: quad.border.color,
            border_radius: quad.border.radius.into(),
            border_width: quad.border.width,
        }
    }
}

impl From<GradientQuad> for GradientQuadPrimitive {
    fn from(q: GradientQuad) -> GradientQuadPrimitive {
        q.packed()
    }
}

impl<'a> From<&'a GradientQuad> for GradientQuadPrimitive {
    fn from(q: &'a GradientQuad) -> GradientQuadPrimitive {
        q.packed()
    }
}

impl From<GradientQuadBuilder> for GradientQuadPrimitive {
    fn from(q: GradientQuadBuilder) -> GradientQuadPrimitive {
        q.build().packed()
    }
}

impl From<GradientQuadBuilder> for GradientQuad {
    fn from(q: GradientQuadBuilder) -> GradientQuad {
        q.build()
    }
}
