use euclid::default::{Point2D, Size2D, Transform2D};

pub mod color;
mod context;
pub mod math;
pub mod mesh;
mod paint;
pub mod pipeline;

pub use color::Color;
pub use context::{Context, ContextRef};
pub use paint::Paint;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Winding {
    /// Winding for solid shapes (CCW)
    Solid = 0,
    /// Winding for holes (CW)
    Hole,
}

impl Winding {
    pub(crate) fn from_u8(w: u8) -> Self {
        if w == 0 {
            Self::Solid
        } else {
            Self::Hole
        }
    }
}

#[repr(i32)]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LineCap {
    #[default]
    Butt = 0,
    Round,
    Square,
    Bevel,
    Miter,
}

#[repr(i32)]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LineJoin {
    #[default]
    Butt = 0,
    Round,
    Square,
    Bevel,
    Miter,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Align: i32 {
        const HALIGN_LEFT = 1 << 0;
        const HALIGN_CENTER = 1 << 1;
        const HALIGN_RIGHT = 1 << 2;

        const VALIGN_TOP = 1 << 3;
        const VALIGN_CENTER = 1 << 4;
        const VALIGN_BOTTOM = 1 << 5;
        const VALIGN_BASELINE = 1 << 6;

        const CENTER = Self::HALIGN_CENTER.bits() | Self::VALIGN_CENTER.bits();
        const DEFAULT = Self::HALIGN_LEFT.bits() | Self::VALIGN_BASELINE.bits();
    }
}

impl Default for Align {
    fn default() -> Self {
        Self::DEFAULT
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct BlendFactor: i32 {
        const ZERO = 1 << 0;
        const ONE = 1 << 1;
        const SRC_COLOR = 1 << 2;
        const ONE_MINUS_SRC_COLOR = 1 << 3;
        const DST_COLOR = 1 << 4;
        const ONE_MINUS_DST_COLOR = 1 << 5;
        const SRC_ALPHA = 1 << 6;
        const ONE_MINUS_SRC_ALPHA = 1 << 7;
        const DST_ALPHA = 1 << 8;
        const ONE_MINUS_DST_ALPHA = 1 << 9;
        const SRC_ALPHA_SATURATE = 1 << 10;
    }
}

impl Default for BlendFactor {
    fn default() -> Self {
        Self::empty()
    }
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CompositeOperation {
    SourceOver = 0,
    SourceIn,
    SourceOut,
    Atop,
    DestinationOver,
    DestinationIn,
    DestinationOut,
    DestinationAtop,
    Lighter,
    Copy,
    Xor,
}

impl CompositeOperation {
    pub fn state(&self) -> CompositeOperationState {
        let (s_factor, d_factor) = match self {
            CompositeOperation::SourceOver => (BlendFactor::ONE, BlendFactor::ONE_MINUS_SRC_ALPHA),
            CompositeOperation::SourceIn => (BlendFactor::DST_ALPHA, BlendFactor::ZERO),
            CompositeOperation::SourceOut => (BlendFactor::ONE_MINUS_DST_ALPHA, BlendFactor::ZERO),
            CompositeOperation::Atop => (BlendFactor::DST_ALPHA, BlendFactor::ONE_MINUS_SRC_ALPHA),
            CompositeOperation::DestinationOver => {
                (BlendFactor::ONE_MINUS_DST_ALPHA, BlendFactor::ONE)
            }
            CompositeOperation::DestinationIn => (BlendFactor::ZERO, BlendFactor::SRC_ALPHA),
            CompositeOperation::DestinationOut => {
                (BlendFactor::ZERO, BlendFactor::ONE_MINUS_SRC_ALPHA)
            }
            CompositeOperation::DestinationAtop => {
                (BlendFactor::ONE_MINUS_DST_ALPHA, BlendFactor::SRC_ALPHA)
            }
            CompositeOperation::Lighter => (BlendFactor::ONE, BlendFactor::ONE),
            CompositeOperation::Copy => (BlendFactor::ONE, BlendFactor::ZERO),
            CompositeOperation::Xor => (
                BlendFactor::ONE_MINUS_DST_ALPHA,
                BlendFactor::ONE_MINUS_SRC_ALPHA,
            ),
        };

        CompositeOperationState {
            src_rgb: s_factor,
            dst_rgb: d_factor,
            src_alpha: s_factor,
            dst_alpha: d_factor,
        }
    }
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CompositeOperationState {
    src_rgb: BlendFactor,
    dst_rgb: BlendFactor,
    src_alpha: BlendFactor,
    dst_alpha: BlendFactor,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct ImageFlags: i32 {
        /// Generate mipmaps during creation of the image
        const GENERATE_MIPMAPS = 1 << 0;
        /// Repeat image in X direction
        const REPEAT_X = 1 << 1;
        /// Repeat image in Y direction
        const REPEAT_Y = 1 << 2;
        /// Flips (inverses) image in Y direction when rendered
        const FLIP_Y = 1 << 3;
        /// Image data has premultiplied alpha
        const PREMULTIPLIED = 1 << 4;
        /// Image interpolation is Nearest instead Linear
        const NEAREST = 1 << 5;
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct TextureType: i32 {
        const ALPHA = 1 << 0;
        const RGBA = 1 << 1;
    }
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct Scissor {
    pub xform: Transform2D<f32>,
    pub extent: Size2D<f32>,
}

impl Scissor {
    pub fn new() -> Self {
        Self {
            xform: Transform2D::identity(),
            extent: Size2D::new(-1.0, -1.0),
        }
    }
}

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: Point2D<f32>,
    pub uv: Point2D<f32>,
}

impl Vertex {
    pub const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2];

    pub const fn new(pos: Point2D<f32>, uv: Point2D<f32>) -> Self {
        Self { pos, uv }
    }

    pub const fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

#[inline]
/// A shorthand for `Vertex::new(pos, uv)`
pub const fn vert(pos: Point2D<f32>, uv: Point2D<f32>) -> Vertex {
    Vertex::new(pos, uv)
}
