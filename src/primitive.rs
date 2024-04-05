#[cfg(all(any(feature = "mesh", feature = "tessellation"), feature = "gradient"))]
use crate::mesh::GradientMeshPrimitive;
#[cfg(any(feature = "mesh", feature = "tessellation"))]
use crate::mesh::{MeshPrimitive, SolidMeshPrimitive};

#[cfg(all(feature = "quad", feature = "gradient"))]
use crate::quad::{GradientQuad, GradientQuadPrimitive};
#[cfg(feature = "quad")]
use crate::quad::{QuadPrimitive, SolidQuad, SolidQuadPrimitive};

#[cfg(feature = "text")]
use crate::text::TextPrimitive;

#[cfg(feature = "image")]
use crate::image::ImagePrimitive;

#[cfg(feature = "custom-primitive")]
use crate::pipeline::CustomPrimitive;

#[derive(Debug, Clone, PartialEq)]
pub enum Primitive {
    #[cfg(feature = "quad")]
    SolidQuad(SolidQuadPrimitive),
    #[cfg(all(feature = "quad", feature = "gradient"))]
    GradientQuad(GradientQuadPrimitive),

    #[cfg(any(feature = "mesh", feature = "tessellation"))]
    SolidMesh(SolidMeshPrimitive),
    #[cfg(all(any(feature = "mesh", feature = "tessellation"), feature = "gradient"))]
    GradientMesh(GradientMeshPrimitive),

    #[cfg(feature = "text")]
    Text(TextPrimitive),

    #[cfg(feature = "image")]
    Image(ImagePrimitive),

    #[cfg(feature = "custom-primitive")]
    Custom(CustomPrimitive),
}

#[cfg(feature = "quad")]
impl From<SolidQuadPrimitive> for Primitive {
    fn from(p: SolidQuadPrimitive) -> Self {
        Primitive::SolidQuad(p)
    }
}

#[cfg(all(feature = "quad", feature = "gradient"))]
impl From<GradientQuadPrimitive> for Primitive {
    fn from(p: GradientQuadPrimitive) -> Self {
        Primitive::GradientQuad(p)
    }
}

#[cfg(feature = "quad")]
impl From<SolidQuad> for Primitive {
    fn from(p: SolidQuad) -> Self {
        Primitive::SolidQuad(SolidQuadPrimitive::new(&p))
    }
}

#[cfg(all(feature = "quad", feature = "gradient"))]
impl From<GradientQuad> for Primitive {
    fn from(p: GradientQuad) -> Self {
        Primitive::GradientQuad(GradientQuadPrimitive::new(&p))
    }
}

#[cfg(feature = "quad")]
impl From<QuadPrimitive> for Primitive {
    fn from(p: QuadPrimitive) -> Self {
        match p {
            QuadPrimitive::Solid(p) => p.into(),
            #[cfg(feature = "gradient")]
            QuadPrimitive::Gradient(p) => p.into(),
            #[cfg(not(feature = "gradient"))]
            _ => unreachable!(),
        }
    }
}

#[cfg(any(feature = "mesh", feature = "tessellation"))]
impl From<SolidMeshPrimitive> for Primitive {
    fn from(p: SolidMeshPrimitive) -> Self {
        Primitive::SolidMesh(p)
    }
}

#[cfg(all(any(feature = "mesh", feature = "tessellation"), feature = "gradient"))]
impl From<GradientMeshPrimitive> for Primitive {
    fn from(p: GradientMeshPrimitive) -> Self {
        Primitive::GradientMesh(p)
    }
}

#[cfg(any(feature = "mesh", feature = "tessellation"))]
impl From<MeshPrimitive> for Primitive {
    fn from(p: MeshPrimitive) -> Self {
        match p {
            MeshPrimitive::Solid(p) => p.into(),
            #[cfg(feature = "gradient")]
            MeshPrimitive::Gradient(p) => p.into(),
            #[cfg(not(feature = "gradient"))]
            _ => unreachable!(),
        }
    }
}

#[cfg(feature = "text")]
impl From<TextPrimitive> for Primitive {
    fn from(p: TextPrimitive) -> Self {
        Primitive::Text(p)
    }
}

#[cfg(feature = "image")]
impl From<ImagePrimitive> for Primitive {
    fn from(p: ImagePrimitive) -> Self {
        Primitive::Image(p)
    }
}

#[cfg(feature = "custom-primitive")]
impl From<CustomPrimitive> for Primitive {
    fn from(p: CustomPrimitive) -> Self {
        Primitive::Custom(p)
    }
}
