//! Draw triangles!
use bytemuck::{Pod, Zeroable};
use std::rc::Rc;

use rootvg_core::color::PackedSrgb;
use rootvg_core::math::{Angle, Point, Transform, Vector};

use super::{Indexed, MeshUniforms};

/// A low-level primitive to render a mesh of triangles with a solid color.
#[derive(Debug, Clone, PartialEq)]
pub struct SolidMesh {
    /// The vertices and indices of the mesh.
    pub buffers: Indexed<SolidVertex2D>,
}

impl SolidMesh {
    pub fn new() -> Self {
        Self {
            buffers: Indexed::new(),
        }
    }
}

/// A two-dimensional vertex with a color.
#[derive(Copy, Clone, Debug, PartialEq, Zeroable, Pod)]
#[repr(C)]
pub struct SolidVertex2D {
    /// The vertex position in 2D space.
    pub position: [f32; 2],

    /// The color of the vertex in __linear__ RGBA.
    pub color: PackedSrgb,
}

impl SolidVertex2D {
    pub fn new(position: impl Into<[f32; 2]>, color: impl Into<PackedSrgb>) -> Self {
        Self {
            position: position.into(),
            color: color.into(),
        }
    }
}

#[derive(Debug)]
pub struct SolidMeshPrimitive {
    pub mesh: Rc<SolidMesh>,
    pub uniform: MeshUniforms,
}

impl SolidMeshPrimitive {
    pub fn new(mesh: &Rc<SolidMesh>) -> Self {
        Self {
            mesh: Rc::clone(mesh),
            uniform: MeshUniforms::default(),
        }
    }

    pub fn new_with_offset(mesh: &Rc<SolidMesh>, offset: Point) -> Self {
        Self {
            mesh: Rc::clone(mesh),
            uniform: MeshUniforms {
                offset: offset.into(),
                ..Default::default()
            },
        }
    }

    pub fn new_with_rotation(
        mesh: &Rc<SolidMesh>,
        angle: Angle,
        rotation_origin: Point,
        offset: Point,
    ) -> Self {
        let transform = Transform::translation(-rotation_origin.x, -rotation_origin.y)
            .then_rotate(angle)
            .then_translate(Vector::new(rotation_origin.x, rotation_origin.y));

        Self::new_with_transform(mesh, offset, transform)
    }

    pub fn new_with_transform(mesh: &Rc<SolidMesh>, offset: Point, transform: Transform) -> Self {
        Self {
            mesh: Rc::clone(mesh),
            uniform: MeshUniforms::new(offset, Some(transform)),
        }
    }
}

impl Clone for SolidMeshPrimitive {
    fn clone(&self) -> Self {
        Self {
            mesh: Rc::clone(&self.mesh),
            uniform: self.uniform,
        }
    }
}

impl PartialEq for SolidMeshPrimitive {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.mesh, &other.mesh) && self.uniform == other.uniform
    }
}
