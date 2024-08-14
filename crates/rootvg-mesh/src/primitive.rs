//! Draw triangles!
use rootvg_core::math::{Angle, Point, Transform, Vector};

mod solid;
pub use solid::*;

#[cfg(feature = "gradient")]
mod gradient;
#[cfg(feature = "gradient")]
pub use gradient::*;

/// A set of vertices and indices representing a list of triangles.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Indexed<T> {
    /// The vertices of the mesh
    pub vertices: Vec<T>,

    /// The list of vertex indices that defines the triangles of the mesh.
    ///
    /// Therefore, this list should always have a length that is a multiple of 3.
    pub indices: Vec<u32>,
}

impl<T> Default for Indexed<T> {
    fn default() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }
}

impl<T> Indexed<T> {
    pub fn new() -> Self {
        Self::default()
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshUniforms {
    /// A 2d transform represented by a column-major 3 by 3 matrix, compressed down
    /// to 3 by 2.
    ///
    /// Note that `size` is not included in the `transform`.
    pub transform: [f32; 6],

    /// The offset in logical points.
    pub offset: [f32; 2],

    /// Whether or not to apply the `transform` matrix. This is used to optimize
    /// meshes with no transformations.
    ///
    /// Note that `size` is not included in the `transform`.
    ///
    /// By default this is set to `0` (false).
    pub has_transform: u32,

    /// Whether or not to snap vertices to the nearest physical pixel to preserve
    /// perceived sharpness.
    ///
    /// By default this is set to `0` (false).
    pub snap_to_nearest_pixel: u32,
}

impl MeshUniforms {
    pub fn new(offset: Vector, transform: Option<Transform>, snap_to_nearest_pixel: bool) -> Self {
        let (transform, has_transform) = if let Some(transform) = transform {
            (transform.to_array(), 1)
        } else {
            ([0.0; 6], 0)
        };

        Self {
            offset: offset.into(),
            transform,
            has_transform,
            snap_to_nearest_pixel: if snap_to_nearest_pixel { 1 } else { 0 },
        }
    }
}

impl Default for MeshUniforms {
    fn default() -> Self {
        Self {
            offset: [0.0; 2],
            transform: [0.0; 6],
            has_transform: 0,
            snap_to_nearest_pixel: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MeshPrimitive {
    Solid(SolidMeshPrimitive),
    #[cfg(feature = "gradient")]
    Gradient(GradientMeshPrimitive),
}

impl MeshPrimitive {
    pub fn set_offset(&mut self, offset: Vector) {
        match self {
            MeshPrimitive::Solid(mesh) => mesh.uniform.offset = offset.into(),
            #[cfg(feature = "gradient")]
            MeshPrimitive::Gradient(mesh) => mesh.uniform.offset = offset.into(),
        }
    }

    pub fn set_rotation(&mut self, angle: Angle, rotation_origin: Point) {
        let transform = Transform::translation(-rotation_origin.x, -rotation_origin.y)
            .then_rotate(angle)
            .then_translate(Vector::new(rotation_origin.x, rotation_origin.y));

        self.set_transform(transform);
    }

    pub fn set_transform(&mut self, transform: Transform) {
        match self {
            MeshPrimitive::Solid(mesh) => {
                mesh.uniform.transform = transform.to_array();
                mesh.uniform.has_transform = 1;
            }
            #[cfg(feature = "gradient")]
            MeshPrimitive::Gradient(mesh) => {
                mesh.uniform.transform = transform.to_array();
                mesh.uniform.has_transform = 1;
            }
        }
    }

    pub fn snap_to_nearest_pixel(&mut self, snap: bool) {
        match self {
            MeshPrimitive::Solid(mesh) => mesh.snap_to_nearest_pixel(snap),
            #[cfg(feature = "gradient")]
            MeshPrimitive::Gradient(mesh) => mesh.snap_to_nearest_pixel(snap),
        }
    }
}
