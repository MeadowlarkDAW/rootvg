use euclid::default::{Point2D, Size2D, Transform2D};
use euclid::Angle;

pub mod color;
mod context;
pub mod math;

pub use color::Color;
pub use context::Context;

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct Paint {
    xform: Transform2D<f32>,
    extent: Size2D<f32>,
    radius: f32,
    feather: f32,
    inner_color: Color,
    outer_color: Color,
    image: i32,
}

impl Paint {
    pub fn new(color: Color) -> Self {
        Self {
            xform: Transform2D::identity(),
            extent: Size2D::zero(),
            radius: 0.0,
            feather: 1.0,
            inner_color: color,
            outer_color: color,
            image: 0,
        }
    }
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Winding {
    /// Winding for solid shapes
    CCW = 1,
    /// Winding for holes
    CW,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Solidity {
    /// CCW
    Solid = 1,
    /// CW
    Hole,
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
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub x: f32,
    pub y: f32,
    pub u: f32,
    pub v: f32,
}

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct Path {
    first: u32,
    count: u32,
    closed: bool,
    num_bevel: u32,
    fill: Vec<Vertex>,
    stroke: Vec<Vertex>,
    winding: Winding,
    convex: bool,
}

impl Path {
    pub(crate) fn new(first: u32) -> Self {
        Self {
            first,
            count: 0,
            closed: false,
            num_bevel: 0,
            fill: Vec::new(),
            stroke: Vec::new(),
            winding: Winding::CCW,
            convex: false,
        }
    }
}
