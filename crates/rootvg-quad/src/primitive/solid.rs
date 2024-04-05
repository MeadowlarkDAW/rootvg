use bytemuck::{Pod, Zeroable};

use rootvg_core::color::PackedSrgb;
use rootvg_core::math::{Point, Rect, Size};

use crate::border::Border;
use crate::shadow::Shadow;
use crate::Radius;

/// A quad primitive with a solid background.
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct SolidQuad {
    /// The bounds of the quad in logical points.
    pub bounds: Rect,
    /// The background color of the quad
    pub bg_color: PackedSrgb,
    /// The [`Border`] of the quad
    pub border: Border,
    /// The shadow of the quad
    pub shadow: Shadow,
}

impl SolidQuad {
    pub fn packed(&self) -> SolidQuadPrimitive {
        SolidQuadPrimitive {
            color: self.bg_color,
            position: self.bounds.origin.into(),
            size: self.bounds.size.into(),
            border_color: self.border.color,
            border_radius: self.border.radius.into(),
            border_width: self.border.width,
            shadow_color: self.shadow.color,
            shadow_offset: self.shadow.offset.into(),
            shadow_blur_radius: self.shadow.blur_radius,
        }
    }

    pub fn builder(size: Size) -> SolidQuadBuilder {
        SolidQuadBuilder::new(size)
    }
}

pub struct SolidQuadBuilder {
    quad: SolidQuad,
}

impl SolidQuadBuilder {
    pub fn new(size: Size) -> Self {
        Self {
            quad: SolidQuad {
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

    pub fn bg_color(mut self, color: impl Into<PackedSrgb>) -> Self {
        self.quad.bg_color = color.into();
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

    pub fn shadow_color(mut self, color: impl Into<PackedSrgb>) -> Self {
        self.quad.shadow.color = color.into();
        self
    }

    pub fn shadow_offset(mut self, offset: Point) -> Self {
        self.quad.shadow.offset = offset;
        self
    }

    pub fn shadow_blur_radius(mut self, blur_radius: f32) -> Self {
        self.quad.shadow.blur_radius = blur_radius;
        self
    }

    pub fn shadow(mut self, shadow: Shadow) -> Self {
        self.quad.shadow = shadow;
        self
    }

    pub fn build(self) -> SolidQuad {
        self.quad
    }
}

/// A quad primitive with a solid background, packed into a format for
/// use in rendering.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct SolidQuadPrimitive {
    /// The background color data of the quad.
    pub color: PackedSrgb,

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

    /// The shadow color of the [`Quad`].
    pub shadow_color: PackedSrgb,

    /// The shadow offset of the [`Quad`] in logical points.
    pub shadow_offset: [f32; 2],

    /// The shadow blur radius of the [`Quad`] in logical points.
    pub shadow_blur_radius: f32,
}

impl SolidQuadPrimitive {
    pub fn new(quad: &SolidQuad) -> Self {
        Self {
            color: quad.bg_color,
            position: quad.bounds.origin.into(),
            size: quad.bounds.size.into(),
            border_color: quad.border.color,
            border_radius: quad.border.radius.into(),
            border_width: quad.border.width,
            shadow_color: quad.shadow.color,
            shadow_offset: quad.shadow.offset.into(),
            shadow_blur_radius: quad.shadow.blur_radius,
        }
    }
}

impl From<SolidQuad> for SolidQuadPrimitive {
    fn from(q: SolidQuad) -> SolidQuadPrimitive {
        q.packed()
    }
}

impl<'a> From<&'a SolidQuad> for SolidQuadPrimitive {
    fn from(q: &'a SolidQuad) -> SolidQuadPrimitive {
        q.packed()
    }
}

impl From<SolidQuadBuilder> for SolidQuadPrimitive {
    fn from(q: SolidQuadBuilder) -> SolidQuadPrimitive {
        q.build().packed()
    }
}

impl From<SolidQuadBuilder> for SolidQuad {
    fn from(q: SolidQuadBuilder) -> SolidQuad {
        q.build()
    }
}
