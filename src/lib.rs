#[cfg(feature = "default-surface")]
pub mod surface;

mod canvas;
mod primitive;
mod primitive_group;

pub mod error;

pub use canvas::{Canvas, CanvasCtx};
pub use primitive::Primitive;
pub use primitive_group::PrimitiveGroup;

pub use rootvg_core::*;

#[cfg(feature = "image")]
pub use rootvg_image as image;

#[cfg(any(feature = "mesh", feature = "tessellation"))]
pub use rootvg_mesh as mesh;

#[cfg(feature = "msaa")]
pub use rootvg_msaa as msaa;

#[cfg(feature = "quad")]
pub use rootvg_quad as quad;

#[cfg(feature = "tessellation")]
pub use rootvg_tessellation as tessellation;

#[cfg(feature = "text")]
pub use rootvg_text as text;
