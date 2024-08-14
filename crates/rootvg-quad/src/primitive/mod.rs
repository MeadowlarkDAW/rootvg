mod solid;
pub use solid::*;

#[cfg(feature = "gradient")]
mod gradient;
#[cfg(feature = "gradient")]
pub use gradient::*;

#[derive(Debug, Clone, PartialEq)]
pub enum QuadPrimitive {
    Solid(SolidQuadPrimitive),
    #[cfg(feature = "gradient")]
    Gradient(GradientQuadPrimitive),
}

bitflags::bitflags! {
    /// Additional flags for a quad primitive.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct QuadFlags: u32 {
        /// In the shader, snap the edges of the quad to
        /// the nearest physical pixel to preserve perceived sharpness.
        const SNAP_EDGES_TO_NEAREST_PIXEL = 0b0001;
        /// In the shader, snap the border width to the nearest physical
        /// pixel to preserve perceived sharpness.
        const SNAP_BORDER_WIDTH_TO_NEAREST_PIXEL = 0b0010;

        /// In the shader, snap the edges and the border width of the
        /// quad to the nearest physical pixel to preserve perceived
        /// sharpness.
        const SNAP_ALL_TO_NEAREST_PIXEL = Self::SNAP_EDGES_TO_NEAREST_PIXEL.bits() | Self::SNAP_BORDER_WIDTH_TO_NEAREST_PIXEL.bits();
    }
}

impl Default for QuadFlags {
    fn default() -> Self {
        QuadFlags::empty()
    }
}
