use rootvg_core::math::ZIndex;
use smallvec::{smallvec, SmallVec};

use crate::{math::RectI32, Primitive};

#[cfg(all(any(feature = "mesh", feature = "tessellation"), feature = "gradient"))]
use crate::mesh::GradientMeshPrimitive;
#[cfg(any(feature = "mesh", feature = "tessellation"))]
use crate::mesh::{MeshPrimitive, SolidMeshPrimitive};

#[cfg(all(feature = "quad", feature = "gradient"))]
use crate::quad::GradientQuadPrimitive;
#[cfg(feature = "quad")]
use crate::quad::SolidQuadPrimitive;

#[cfg(feature = "text")]
use crate::text::TextPrimitive;

#[cfg(feature = "image")]
use crate::image::ImagePrimitive;

#[cfg(feature = "custom-primitive")]
use crate::pipeline::CustomPrimitive;

const STATIC_ALLOC_PRIMITIVES: usize = 4;

/// A group of primitives that can be added to a canvas. This is usally
/// the output of a single widget.
#[derive(Debug, Clone, PartialEq)]
pub struct PrimitiveGroup {
    pub(crate) primitive_batches: SmallVec<[PrimitiveBatchSlice; STATIC_ALLOC_PRIMITIVES]>,
    current_scissor_rect: Option<RectI32>,
    current_z_index: ZIndex,
    create_new_batch: bool,
}

impl Default for PrimitiveGroup {
    fn default() -> Self {
        Self {
            primitive_batches: SmallVec::new(),
            current_scissor_rect: None,
            current_z_index: 0,
            create_new_batch: true,
        }
    }
}

impl PrimitiveGroup {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.primitive_batches.clear();
        self.current_scissor_rect = None;
        self.create_new_batch = true;
        self.current_z_index = 0;
    }

    pub fn set_scissor_rect(&mut self, scissor_rect: RectI32) {
        self.current_scissor_rect = Some(scissor_rect);
        self.create_new_batch = true;
    }

    pub fn reset_scissor_rect(&mut self) {
        self.current_scissor_rect = None;
        self.create_new_batch = true;
    }

    pub fn scissor_rect(&self) -> &Option<RectI32> {
        &self.current_scissor_rect
    }

    pub fn set_z_index(&mut self, z_index: ZIndex) {
        if self.current_z_index != z_index {
            self.create_new_batch = true;
        }
        self.current_z_index = z_index;
    }

    pub fn z_index(&self) -> ZIndex {
        self.current_z_index
    }

    pub fn add(&mut self, primitive: impl Into<Primitive>) {
        let primitive: Primitive = primitive.into();

        match primitive {
            #[cfg(feature = "quad")]
            Primitive::SolidQuad(p) => self.add_solid_quad(p),
            #[cfg(all(feature = "quad", feature = "gradient"))]
            Primitive::GradientQuad(p) => self.add_gradient_quad(p),

            #[cfg(any(feature = "mesh", feature = "tessellation"))]
            Primitive::SolidMesh(p) => self.add_solid_mesh(p),
            #[cfg(all(any(feature = "mesh", feature = "tessellation"), feature = "gradient"))]
            Primitive::GradientMesh(p) => self.add_gradient_mesh(p),

            #[cfg(feature = "text")]
            Primitive::Text(p) => self.add_text(p),

            #[cfg(feature = "image")]
            Primitive::Image(p) => self.add_image(p),

            #[cfg(feature = "custom-primitive")]
            Primitive::Custom(p) => self.add_custom_primitive(p),
        }
    }

    #[cfg(feature = "quad")]
    pub fn add_solid_quad(&mut self, quad: impl Into<SolidQuadPrimitive>) {
        let quad = quad.into();

        if self.create_new_batch {
            self.create_new_batch = false;

            self.primitive_batches.push(PrimitiveBatchSlice {
                z_index: self.current_z_index,
                scissor_rect: self.current_scissor_rect,
                kind: PrimitiveBatchKind::SolidQuad(smallvec![quad]),
            });
        } else {
            // `self.create_new_batch` is never `false` when `self.primitve_batches` is empty
            let last_batch = self.primitive_batches.last_mut().unwrap();

            if let PrimitiveBatchKind::SolidQuad(batch) = &mut last_batch.kind {
                batch.push(quad);
            } else {
                self.primitive_batches.push(PrimitiveBatchSlice {
                    z_index: self.current_z_index,
                    scissor_rect: self.current_scissor_rect,
                    kind: PrimitiveBatchKind::SolidQuad(smallvec![quad]),
                });
            }
        };
    }

    #[cfg(all(feature = "quad", feature = "gradient"))]
    pub fn add_gradient_quad(&mut self, quad: impl Into<GradientQuadPrimitive>) {
        let quad = quad.into();

        if self.create_new_batch {
            self.create_new_batch = false;

            self.primitive_batches.push(PrimitiveBatchSlice {
                z_index: self.current_z_index,
                scissor_rect: self.current_scissor_rect,
                kind: PrimitiveBatchKind::GradientQuad(smallvec![quad]),
            });
        } else {
            // `self.create_new_batch` is never `false` when `self.primitve_batches` is empty
            let last_batch = self.primitive_batches.last_mut().unwrap();

            if let PrimitiveBatchKind::GradientQuad(batch) = &mut last_batch.kind {
                batch.push(quad);
            } else {
                self.primitive_batches.push(PrimitiveBatchSlice {
                    z_index: self.current_z_index,
                    scissor_rect: self.current_scissor_rect,
                    kind: PrimitiveBatchKind::GradientQuad(smallvec![quad]),
                });
            }
        };
    }

    #[cfg(any(feature = "mesh", feature = "tessellation"))]
    pub fn add_solid_mesh(&mut self, mesh: impl Into<SolidMeshPrimitive>) {
        let mesh: SolidMeshPrimitive = mesh.into();

        if self.create_new_batch {
            self.create_new_batch = false;

            self.primitive_batches.push(PrimitiveBatchSlice {
                z_index: self.current_z_index,
                scissor_rect: self.current_scissor_rect,
                kind: PrimitiveBatchKind::SolidMesh(smallvec![mesh]),
            });
        } else {
            // `self.create_new_batch` is never `false` when `self.primitve_batches` is empty
            let last_batch = self.primitive_batches.last_mut().unwrap();

            if let PrimitiveBatchKind::SolidMesh(batch) = &mut last_batch.kind {
                batch.push(mesh);
            } else {
                self.primitive_batches.push(PrimitiveBatchSlice {
                    z_index: self.current_z_index,
                    scissor_rect: self.current_scissor_rect,
                    kind: PrimitiveBatchKind::SolidMesh(smallvec![mesh]),
                });
            }
        }
    }

    #[cfg(all(any(feature = "mesh", feature = "tessellation"), feature = "gradient"))]
    pub fn add_gradient_mesh(&mut self, mesh: impl Into<GradientMeshPrimitive>) {
        let mesh: GradientMeshPrimitive = mesh.into();

        if self.create_new_batch {
            self.create_new_batch = false;

            self.primitive_batches.push(PrimitiveBatchSlice {
                z_index: self.current_z_index,
                scissor_rect: self.current_scissor_rect,
                kind: PrimitiveBatchKind::GradientMesh(smallvec![mesh]),
            });
        } else {
            // `self.create_new_batch` is never `false` when `self.primitve_batches` is empty
            let last_batch = self.primitive_batches.last_mut().unwrap();

            if let PrimitiveBatchKind::GradientMesh(batch) = &mut last_batch.kind {
                batch.push(mesh);
            } else {
                self.primitive_batches.push(PrimitiveBatchSlice {
                    z_index: self.current_z_index,
                    scissor_rect: self.current_scissor_rect,
                    kind: PrimitiveBatchKind::GradientMesh(smallvec![mesh]),
                });
            }
        }
    }

    #[cfg(any(feature = "mesh", feature = "tessellation"))]
    pub fn add_mesh(&mut self, mesh: MeshPrimitive) {
        match mesh {
            MeshPrimitive::Solid(mesh) => {
                self.add_solid_mesh(mesh);
            }
            #[cfg(feature = "gradient")]
            MeshPrimitive::Gradient(mesh) => {
                self.add_gradient_mesh(mesh);
            }
            #[cfg(not(feature = "gradient"))]
            _ => unreachable!(),
        }
    }

    #[cfg(feature = "text")]
    pub fn add_text(&mut self, text: TextPrimitive) {
        if self.create_new_batch {
            self.create_new_batch = false;

            self.primitive_batches.push(PrimitiveBatchSlice {
                z_index: self.current_z_index,
                scissor_rect: self.current_scissor_rect,
                kind: PrimitiveBatchKind::Text(smallvec![text]),
            });
        } else {
            // `self.create_new_batch` is never `false` when `self.primitve_batches` is empty
            let last_batch = self.primitive_batches.last_mut().unwrap();

            if let PrimitiveBatchKind::Text(batch) = &mut last_batch.kind {
                batch.push(text);
            } else {
                self.primitive_batches.push(PrimitiveBatchSlice {
                    z_index: self.current_z_index,
                    scissor_rect: self.current_scissor_rect,
                    kind: PrimitiveBatchKind::Text(smallvec![text]),
                });
            }
        }
    }

    #[cfg(feature = "image")]
    pub fn add_image(&mut self, image: impl Into<ImagePrimitive>) {
        let image: ImagePrimitive = image.into();

        if self.create_new_batch {
            self.create_new_batch = false;

            self.primitive_batches.push(PrimitiveBatchSlice {
                z_index: self.current_z_index,
                scissor_rect: self.current_scissor_rect,
                kind: PrimitiveBatchKind::Image(smallvec![image]),
            });
        } else {
            // `self.create_new_batch` is never `false` when `self.primitve_batches` is empty
            let last_batch = self.primitive_batches.last_mut().unwrap();

            if let PrimitiveBatchKind::Image(batch) = &mut last_batch.kind {
                batch.push(image);
            } else {
                self.primitive_batches.push(PrimitiveBatchSlice {
                    z_index: self.current_z_index,
                    scissor_rect: self.current_scissor_rect,
                    kind: PrimitiveBatchKind::Image(smallvec![image]),
                });
            }
        }
    }

    #[cfg(feature = "custom-primitive")]
    pub fn add_custom_primitive(&mut self, primitive: CustomPrimitive) {
        if self.create_new_batch {
            self.create_new_batch = false;

            self.primitive_batches.push(PrimitiveBatchSlice {
                z_index: self.current_z_index,
                scissor_rect: self.current_scissor_rect,
                kind: PrimitiveBatchKind::Custom(smallvec![primitive]),
            });
        } else {
            // `self.create_new_batch` is never `false` when `self.primitve_batches` is empty
            let last_batch = self.primitive_batches.last_mut().unwrap();

            if let PrimitiveBatchKind::Custom(batch) = &mut last_batch.kind {
                batch.push(primitive);
            } else {
                self.primitive_batches.push(PrimitiveBatchSlice {
                    z_index: self.current_z_index,
                    scissor_rect: self.current_scissor_rect,
                    kind: PrimitiveBatchKind::Custom(smallvec![primitive]),
                });
            }
        }
    }

    #[cfg(feature = "quad")]
    pub fn add_solid_quad_batch(
        &mut self,
        quads: impl IntoIterator<Item = impl Into<SolidQuadPrimitive>>,
    ) {
        if self.create_new_batch {
            self.create_new_batch = false;

            self.primitive_batches.push(PrimitiveBatchSlice {
                z_index: self.current_z_index,
                scissor_rect: self.current_scissor_rect,
                kind: PrimitiveBatchKind::SolidQuad(
                    quads.into_iter().map(|quad| quad.into()).collect(),
                ),
            });
        } else {
            // `self.create_new_batch` is never `false` when `self.primitve_batches` is empty
            let last_batch = self.primitive_batches.last_mut().unwrap();

            if let PrimitiveBatchKind::SolidQuad(batch) = &mut last_batch.kind {
                for quad in quads.into_iter() {
                    batch.push(quad.into());
                }
            } else {
                self.primitive_batches.push(PrimitiveBatchSlice {
                    z_index: self.current_z_index,
                    scissor_rect: self.current_scissor_rect,
                    kind: PrimitiveBatchKind::SolidQuad(
                        quads.into_iter().map(|quad| quad.into()).collect(),
                    ),
                });
            }
        }
    }

    #[cfg(all(feature = "quad", feature = "gradient"))]
    pub fn add_gradient_quad_batch(
        &mut self,
        quads: impl IntoIterator<Item = impl Into<GradientQuadPrimitive>>,
    ) {
        if self.create_new_batch {
            self.create_new_batch = false;

            self.primitive_batches.push(PrimitiveBatchSlice {
                z_index: self.current_z_index,
                scissor_rect: self.current_scissor_rect,
                kind: PrimitiveBatchKind::GradientQuad(
                    quads.into_iter().map(|quad| quad.into()).collect(),
                ),
            });
        } else {
            // `self.create_new_batch` is never `false` when `self.primitve_batches` is empty
            let last_batch = self.primitive_batches.last_mut().unwrap();

            if let PrimitiveBatchKind::GradientQuad(batch) = &mut last_batch.kind {
                for quad in quads.into_iter() {
                    batch.push(quad.into());
                }
            } else {
                self.primitive_batches.push(PrimitiveBatchSlice {
                    z_index: self.current_z_index,
                    scissor_rect: self.current_scissor_rect,
                    kind: PrimitiveBatchKind::GradientQuad(
                        quads.into_iter().map(|quad| quad.into()).collect(),
                    ),
                });
            }
        }
    }

    #[cfg(feature = "text")]
    pub fn add_text_batch(&mut self, buffers: impl IntoIterator<Item = TextPrimitive>) {
        if self.create_new_batch {
            self.create_new_batch = false;

            self.primitive_batches.push(PrimitiveBatchSlice {
                z_index: self.current_z_index,
                scissor_rect: self.current_scissor_rect,
                kind: PrimitiveBatchKind::Text(buffers.into_iter().collect()),
            });
        } else {
            // `self.create_new_batch` is never `false` when `self.primitve_batches` is empty
            let last_batch = self.primitive_batches.last_mut().unwrap();

            if let PrimitiveBatchKind::Text(batch) = &mut last_batch.kind {
                for text in buffers.into_iter() {
                    batch.push(text);
                }
            } else {
                self.primitive_batches.push(PrimitiveBatchSlice {
                    z_index: self.current_z_index,
                    scissor_rect: self.current_scissor_rect,
                    kind: PrimitiveBatchKind::Text(buffers.into_iter().collect()),
                });
            }
        }
    }

    #[cfg(any(feature = "mesh", feature = "tessellation"))]
    pub fn add_solid_mesh_batch(&mut self, meshes: impl IntoIterator<Item = SolidMeshPrimitive>) {
        if self.create_new_batch {
            self.create_new_batch = false;

            self.primitive_batches.push(PrimitiveBatchSlice {
                z_index: self.current_z_index,
                scissor_rect: self.current_scissor_rect,
                kind: PrimitiveBatchKind::SolidMesh(meshes.into_iter().collect()),
            });
        } else {
            // `self.create_new_batch` is never `false` when `self.primitve_batches` is empty
            let last_batch = self.primitive_batches.last_mut().unwrap();

            if let PrimitiveBatchKind::SolidMesh(batch) = &mut last_batch.kind {
                for mesh in meshes.into_iter() {
                    batch.push(mesh);
                }
            } else {
                self.primitive_batches.push(PrimitiveBatchSlice {
                    z_index: self.current_z_index,
                    scissor_rect: self.current_scissor_rect,
                    kind: PrimitiveBatchKind::SolidMesh(meshes.into_iter().collect()),
                });
            }
        }
    }

    #[cfg(all(any(feature = "mesh", feature = "tessellation"), feature = "gradient"))]
    pub fn add_gradient_mesh_batch(
        &mut self,
        meshes: impl IntoIterator<Item = GradientMeshPrimitive>,
    ) {
        if self.create_new_batch {
            self.create_new_batch = false;

            self.primitive_batches.push(PrimitiveBatchSlice {
                z_index: self.current_z_index,
                scissor_rect: self.current_scissor_rect,
                kind: PrimitiveBatchKind::GradientMesh(meshes.into_iter().collect()),
            });
        } else {
            // `self.create_new_batch` is never `false` when `self.primitve_batches` is empty
            let last_batch = self.primitive_batches.last_mut().unwrap();

            if let PrimitiveBatchKind::GradientMesh(batch) = &mut last_batch.kind {
                for mesh in meshes.into_iter() {
                    batch.push(mesh);
                }
            } else {
                self.primitive_batches.push(PrimitiveBatchSlice {
                    z_index: self.current_z_index,
                    scissor_rect: self.current_scissor_rect,
                    kind: PrimitiveBatchKind::GradientMesh(meshes.into_iter().collect()),
                });
            }
        }
    }

    #[cfg(any(feature = "mesh", feature = "tessellation"))]
    pub fn add_mesh_batch(&mut self, meshes: impl IntoIterator<Item = MeshPrimitive>) {
        for mesh in meshes.into_iter() {
            self.add_mesh(mesh);
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PrimitiveBatchSlice {
    pub(crate) kind: PrimitiveBatchKind,
    pub(crate) z_index: ZIndex,
    pub(crate) scissor_rect: Option<RectI32>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PrimitiveBatchKind {
    #[cfg(feature = "quad")]
    SolidQuad(SmallVec<[SolidQuadPrimitive; STATIC_ALLOC_PRIMITIVES]>),
    #[cfg(all(feature = "quad", feature = "gradient"))]
    GradientQuad(SmallVec<[GradientQuadPrimitive; STATIC_ALLOC_PRIMITIVES]>),

    #[cfg(feature = "text")]
    Text(SmallVec<[TextPrimitive; STATIC_ALLOC_PRIMITIVES]>),

    #[cfg(any(feature = "mesh", feature = "tessellation"))]
    SolidMesh(SmallVec<[SolidMeshPrimitive; STATIC_ALLOC_PRIMITIVES]>),
    #[cfg(all(any(feature = "mesh", feature = "tessellation"), feature = "gradient"))]
    GradientMesh(SmallVec<[GradientMeshPrimitive; STATIC_ALLOC_PRIMITIVES]>),

    #[cfg(feature = "image")]
    Image(SmallVec<[ImagePrimitive; STATIC_ALLOC_PRIMITIVES]>),

    #[cfg(feature = "custom-primitive")]
    Custom(SmallVec<[CustomPrimitive; STATIC_ALLOC_PRIMITIVES]>),
}
