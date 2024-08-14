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
        /// In the shader, snap the left and right edge of the quad to
        /// the nearest physical pixel to preserve perceived sharpness.
        const SNAP_TO_NEAREST_PIXEL_H = 0b0001;
        /// In the shader, snap the top and bottom edge of the quad to
        /// the nearest physical pixel to preserve perceived sharpness.
        const SNAP_TO_NEAREST_PIXEL_V = 0b0010;
        /// In the shader, snap the border width to the nearest physical
        /// pixel to preserve perceived sharpness.
        const SNAP_BORDER_WIDTH_TO_NEAREST_PIXEL = 0b0100;

        /// In the shader, snap the edges and the border width of the
        /// quad to the nearest physical pixel to preserve perceived
        /// sharpness.
        const SNAP_ALL_TO_NEAREST_PIXEL = Self::SNAP_TO_NEAREST_PIXEL_H.bits() | Self::SNAP_TO_NEAREST_PIXEL_V.bits() | Self::SNAP_BORDER_WIDTH_TO_NEAREST_PIXEL.bits();
    }
}

impl Default for QuadFlags {
    fn default() -> Self {
        QuadFlags::empty()
    }
}
