//! Draw triangles!
use bytemuck::{Pod, Zeroable};
use std::rc::Rc;

use rootvg_core::gradient::PackedGradient;
use rootvg_core::math::{Angle, Point, Transform, Vector};

use super::{Indexed, MeshUniforms};

/// A low-level primitive to render a mesh of triangles with a gradient.
#[derive(Debug, Clone, PartialEq)]
pub struct GradientMesh {
    /// The vertices and indices of the mesh.
    pub buffers: Indexed<GradientVertex2D>,
}

impl GradientMesh {
    pub fn new() -> Self {
        Self {
            buffers: Indexed::new(),
        }
    }
}

/// A vertex which contains 2D position & packed gradient data.
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
#[repr(C)]
pub struct GradientVertex2D {
    /// The vertex position in 2D space.
    pub position: [f32; 2],

    /// The packed vertex data of the gradient.
    pub gradient: PackedGradient,
}

impl GradientVertex2D {
    pub fn new(position: impl Into<[f32; 2]>, color: impl Into<PackedGradient>) -> Self {
        Self {
            position: position.into(),
            gradient: color.into(),
        }
    }
}

#[derive(Debug)]
pub struct GradientMeshPrimitive {
    pub mesh: Rc<GradientMesh>,
    pub uniform: MeshUniforms,
}

impl GradientMeshPrimitive {
    pub fn new(mesh: &Rc<GradientMesh>) -> Self {
        Self {
            mesh: Rc::clone(mesh),
            uniform: MeshUniforms::default(),
        }
    }

    pub fn new_with_offset(mesh: &Rc<GradientMesh>, offset: Point) -> Self {
        Self {
            mesh: Rc::clone(mesh),
            uniform: MeshUniforms {
                offset: offset.into(),
                ..Default::default()
            },
        }
    }

    pub fn new_with_rotation(
        mesh: &Rc<GradientMesh>,
        angle: Angle,
        rotation_origin: Point,
        offset: Point,
    ) -> Self {
        let transform = Transform::translation(-rotation_origin.x, -rotation_origin.y)
            .then_rotate(angle)
            .then_translate(Vector::new(rotation_origin.x, rotation_origin.y));

        Self::new_with_transform(mesh, offset, transform)
    }

    pub fn new_with_transform(
        mesh: &Rc<GradientMesh>,
        offset: Point,
        transform: Transform,
    ) -> Self {
        Self {
            mesh: Rc::clone(mesh),
            uniform: MeshUniforms::new(offset, Some(transform)),
        }
    }
}

impl Clone for GradientMeshPrimitive {
    fn clone(&self) -> Self {
        Self {
            mesh: Rc::clone(&self.mesh),
            uniform: self.uniform,
        }
    }
}

impl PartialEq for GradientMeshPrimitive {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.mesh, &other.mesh) && self.uniform == other.uniform
    }
}
