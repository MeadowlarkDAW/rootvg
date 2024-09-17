use euclid::default::{Size2D, Transform2D};

use crate::{Color, LineCap, LineJoin};

pub const DEFAULT_MITER_LIMIT: f32 = 10.0;
pub const MAX_STROKE_WIDTH: f32 = 10_000.0;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
/// The paint to apply to a mesh's stroke/fill
pub enum Paint {
    /// A single solid color
    SolidColor(Color),
    /// Gradient
    Gradient(GradientPaint),
    /// Image
    Image(ImagePaint),
}

impl Default for Paint {
    fn default() -> Self {
        Self::SolidColor(crate::color::BLACK)
    }
}

impl From<Color> for Paint {
    fn from(c: Color) -> Self {
        Self::SolidColor(c)
    }
}

// TODO: Support more gradient stops.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GradientPaint {
    /// The transformation matrix applied to the gradient
    ///
    /// If this is `None`, then no transform will be used. (This is equivalent
    /// to using `Transform2D::Identity` except that the matrix calculations
    /// can be skipped to improve performance).
    pub transform: Option<Transform2D<f32>>,

    /// The extent of the gradient
    ///
    /// If this is `None`, then the size of the mesh will be used instead.
    pub extent: Option<Size2D<f32>>,

    /// The inner color of the gradient
    pub inner_color: Color,
    /// The outer color of the gradient
    pub outer_color: Color,

    pub radius: f32,
    pub feather: f32,
}

impl From<GradientPaint> for Paint {
    fn from(g: GradientPaint) -> Self {
        Paint::Gradient(g)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImagePaint {
    /// The transformation matrix applied to the image
    ///
    /// If this is `None`, then no transform will be used. (This is equivalent
    /// to using `Transform2D::Identity` except that the matrix calculations
    /// can be skipped to improve performance).
    pub transform: Transform2D<f32>,

    /// The extent of the image
    ///
    /// If this is `None`, then the size of the mesh will be used instead.
    pub extent: Option<Size2D<f32>>,

    /// The ID of the image to sample from
    pub image_id: u32,
}

impl From<ImagePaint> for Paint {
    fn from(i: ImagePaint) -> Self {
        Paint::Image(i)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
/// The options for converting a path into a tessellated mesh.
pub struct MeshOpts {
    /// The stroke width in pixels
    ///
    /// If `stroke_width <= 0.0`, then no stroke will be rendered.
    ///
    /// Defaults to `0.0` (no stroke).
    pub stroke_width: f32,

    /// The miter limit that controls when a sharp corner is beveled
    ///
    /// Defaults to `10.0`.
    pub miter_limit: f32,

    /// The line joining method
    ///
    /// Defaults to [`LineJoin::Butt`].
    pub line_join: LineJoin,

    /// The line capping method
    ///
    /// Defaults to [`LineCap::Butt`].
    pub line_cap: LineCap,

    /// Whether or not closed paths should be filled
    ///
    /// Defaults to `false`.
    pub fill: bool,

    /// Whether or not to tessellate the mesh with antialiasing
    ///
    /// If [`Context`] was initialized with `antialiasing_enabled` set to `true`,
    /// then this is enabled by default.
    ///
    /// Otherwise if [`Context`] was initialized with `antialiasing_enabled` set
    /// to `false`, then this will have no effect.
    ///
    /// Defaults to `true`.
    pub anti_alias: bool,
}

impl Default for MeshOpts {
    fn default() -> Self {
        Self::new()
    }
}

impl MeshOpts {
    pub const fn new() -> Self {
        Self {
            stroke_width: 0.0,
            miter_limit: DEFAULT_MITER_LIMIT,
            line_join: LineJoin::Butt,
            line_cap: LineCap::Butt,
            fill: false,
            anti_alias: true,
        }
    }

    /// The stroke width in pixels
    ///
    /// If `stroke_width <= 0.0`, then no stroke will be rendered.
    ///
    /// Defaults to `0.0` (no stroke).
    pub const fn stroke_width(mut self, width: f32) -> Self {
        self.stroke_width = width;
        self
    }

    /// The miter limit that controls when a sharp corner is beveled
    ///
    /// Defaults to `10.0`.
    pub const fn miter_limit(mut self, limit: f32) -> Self {
        self.miter_limit = limit;
        self
    }

    /// The line joining method
    ///
    /// Defaults to [`LineJoin::Butt`].
    pub const fn line_join(mut self, line_join: LineJoin) -> Self {
        self.line_join = line_join;
        self
    }

    /// The line capping method
    ///
    /// Defaults to [`LineCap::Butt`].
    pub const fn line_cap(mut self, line_cap: LineCap) -> Self {
        self.line_cap = line_cap;
        self
    }

    /// Whether or not closed paths should be filled
    ///
    /// Defaults to `false`.
    pub const fn fill(mut self, fill: bool) -> Self {
        self.fill = fill;
        self
    }

    /// Whether or not to tessellate the mesh with antialiasing
    ///
    /// If [`Context`] was initialized with `antialiasing_enabled` set to `true`,
    /// then this is enabled by default.
    ///
    /// Otherwise if [`Context`] was initialized with `antialiasing_enabled` set
    /// to `false`, then this will have no effect.
    ///
    /// Defaults to `true`.
    pub const fn anti_alias(mut self, anti_alias: bool) -> Self {
        self.anti_alias = anti_alias;
        self
    }
}
