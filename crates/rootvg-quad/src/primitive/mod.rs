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
