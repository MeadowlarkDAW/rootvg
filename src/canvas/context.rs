use crate::math::{RectI32, Vector, VectorI32, ZIndex};
use crate::primitive_group::{PrimitiveBatchKind, PrimitiveGroup};
use crate::Primitive;

use super::{BatchEntry, BatchKey, Canvas};

#[cfg(feature = "custom-primitive")]
use super::QueuedCustomPrimitive;

pub struct CanvasCtx<'a> {
    pub(super) canvas: &'a mut Canvas,
}

impl<'a> CanvasCtx<'a> {
    pub fn set_scissor_rect(&mut self, scissor_rect: RectI32) {
        self.canvas.set_scissor_rect(scissor_rect);
    }

    pub fn reset_scissor_rect(&mut self) {
        self.canvas.reset_scissor_rect();
    }

    pub fn scissor_rect(&self) -> RectI32 {
        self.canvas.scissor_rect
    }

    pub fn set_z_index(&mut self, z_index: ZIndex) {
        self.canvas.z_index = z_index;
    }

    pub fn z_index(&mut self) -> ZIndex {
        self.canvas.z_index
    }

    pub fn add(&mut self, primitive: impl Into<Primitive>) {
        if self.canvas.scissor_rect_out_of_bounds {
            return;
        }

        let key = BatchKey::new(self.canvas.scissor_rect, self.canvas.z_index, 0);
        let batch_entry = self
            .canvas
            .batches
            .entry(key)
            .or_insert_with(|| BatchEntry::new());

        add(
            primitive,
            batch_entry,
            #[cfg(feature = "custom-primitive")]
            self.canvas.num_custom_pipelines,
        );
    }

    pub fn add_with_offset(&mut self, primitive: impl Into<Primitive>, offset: Vector) {
        if self.canvas.scissor_rect_out_of_bounds {
            return;
        }

        let key = BatchKey::new(self.canvas.scissor_rect, self.canvas.z_index, 0);
        let batch_entry = self
            .canvas
            .batches
            .entry(key)
            .or_insert_with(|| BatchEntry::new());

        add_with_offset(
            primitive,
            offset,
            batch_entry,
            #[cfg(feature = "custom-primitive")]
            self.canvas.num_custom_pipelines,
        );
    }

    pub fn add_batch(&mut self, primitives: impl IntoIterator<Item = impl Into<Primitive>>) {
        if self.canvas.scissor_rect_out_of_bounds {
            return;
        }

        let key = BatchKey::new(self.canvas.scissor_rect, self.canvas.z_index, 0);
        let batch_entry = self
            .canvas
            .batches
            .entry(key)
            .or_insert_with(|| BatchEntry::new());

        for primitive in primitives.into_iter() {
            add(
                primitive,
                batch_entry,
                #[cfg(feature = "custom-primitive")]
                self.canvas.num_custom_pipelines,
            );
        }
    }

    pub fn add_batch_with_offset(
        &mut self,
        primitives: impl IntoIterator<Item = impl Into<Primitive>>,
        offset: Vector,
    ) {
        if self.canvas.scissor_rect_out_of_bounds {
            return;
        }

        let key = BatchKey::new(self.canvas.scissor_rect, self.canvas.z_index, 0);
        let batch_entry = self
            .canvas
            .batches
            .entry(key)
            .or_insert_with(|| BatchEntry::new());

        for primitive in primitives.into_iter() {
            add_with_offset(
                primitive,
                offset,
                batch_entry,
                #[cfg(feature = "custom-primitive")]
                self.canvas.num_custom_pipelines,
            );
        }
    }

    pub fn add_group(&mut self, group: &PrimitiveGroup) {
        self.add_group_with_offset(group, Vector::zero());
    }

    pub fn add_group_with_offset(&mut self, group: &PrimitiveGroup, offset: Vector) {
        if self.canvas.scissor_rect_out_of_bounds {
            return;
        }

        for batch in group.primitive_batches.iter() {
            let scissor_rect = if let Some(scissor_rect) = batch.scissor_rect {
                let offset_i32 = VectorI32::new(offset.x.round() as i32, offset.y.round() as i32);

                let Some(c) = super::offset_scissor_rect(
                    scissor_rect,
                    offset_i32,
                    self.canvas.logical_size_i32,
                ) else {
                    // Scissor rect is off screen
                    continue;
                };
                c
            } else {
                self.canvas.scissor_rect
            };

            let key = BatchKey::new(scissor_rect, self.canvas.z_index, batch.z_index);

            let batch_entry = self
                .canvas
                .batches
                .entry(key)
                .or_insert_with(|| BatchEntry::new());

            match &batch.kind {
                #[cfg(feature = "quad")]
                PrimitiveBatchKind::SolidQuad(quads) => {
                    for quad in quads.iter() {
                        let mut quad_copy = *quad;
                        quad_copy.position[0] += offset.x;
                        quad_copy.position[1] += offset.y;

                        batch_entry.solid_quads.push(quad_copy);
                    }
                }
                #[cfg(all(feature = "quad", feature = "gradient"))]
                PrimitiveBatchKind::GradientQuad(quads) => {
                    for quad in quads.iter() {
                        let mut quad_copy = *quad;
                        quad_copy.position[0] += offset.x;
                        quad_copy.position[1] += offset.y;

                        batch_entry.gradient_quads.push(quad_copy);
                    }
                }
                #[cfg(feature = "text")]
                PrimitiveBatchKind::Text(text) => {
                    for t in text.iter() {
                        let mut t_copy = t.clone();
                        t_copy.pos.x += offset.x;
                        t_copy.pos.y += offset.y;

                        batch_entry.text.push(t_copy);
                    }
                }
                #[cfg(any(feature = "mesh", feature = "tessellation"))]
                PrimitiveBatchKind::SolidMesh(meshes) => {
                    for mesh in meshes.iter() {
                        let mut mesh_copy = mesh.clone();

                        mesh_copy.uniform.offset[0] += offset.x;
                        mesh_copy.uniform.offset[1] += offset.y;

                        batch_entry.solid_meshes.push(mesh_copy);
                    }
                }
                #[cfg(all(any(feature = "mesh", feature = "tessellation"), feature = "gradient"))]
                PrimitiveBatchKind::GradientMesh(meshes) => {
                    for mesh in meshes.iter() {
                        let mut mesh_copy = mesh.clone();

                        mesh_copy.uniform.offset[0] += offset.x;
                        mesh_copy.uniform.offset[1] += offset.y;

                        batch_entry.gradient_meshes.push(mesh_copy);
                    }
                }
                #[cfg(feature = "image")]
                PrimitiveBatchKind::Image(images) => {
                    for image in images.iter() {
                        let mut image_copy = image.clone();

                        image_copy.vertex.position[0] += offset.x;
                        image_copy.vertex.position[1] += offset.y;

                        batch_entry.images.push(image_copy);
                    }
                }
                #[cfg(feature = "custom-primitive")]
                PrimitiveBatchKind::Custom(primitives) => {
                    for p in primitives.iter() {
                        let Some(custom_batch) = batch_entry
                            .custom_primitives
                            .get_mut(p.pipeline_index as usize)
                        else {
                            log::error!(
                                "Primitive group had custom primitive with pipeline index {}, but canvas has {} custom pipelines, ignoring primitive",
                                p.pipeline_index,
                                self.canvas.num_custom_pipelines,
                            );

                            continue;
                        };

                        custom_batch.push(QueuedCustomPrimitive {
                            id: p.id,
                            offset: Vector::new(p.offset.x + offset.x, p.offset.y + offset.y),
                        });
                    }
                }
            }
        }
    }
}

fn add(
    primitive: impl Into<Primitive>,
    batch_entry: &mut BatchEntry,
    #[cfg(feature = "custom-primitive")] num_custom_pipelines: usize,
) {
    let primitive: Primitive = primitive.into();

    match primitive {
        #[cfg(feature = "quad")]
        Primitive::SolidQuad(p) => {
            batch_entry.solid_quads.push(p);
        }
        #[cfg(all(feature = "quad", feature = "gradient"))]
        Primitive::GradientQuad(p) => {
            batch_entry.gradient_quads.push(p);
        }

        #[cfg(any(feature = "mesh", feature = "tessellation"))]
        Primitive::SolidMesh(p) => {
            batch_entry.solid_meshes.push(p);
        }
        #[cfg(all(any(feature = "mesh", feature = "tessellation"), feature = "gradient"))]
        Primitive::GradientMesh(p) => {
            batch_entry.gradient_meshes.push(p);
        }

        #[cfg(feature = "text")]
        Primitive::Text(p) => {
            batch_entry.text.push(p);
        }

        #[cfg(feature = "image")]
        Primitive::Image(p) => {
            batch_entry.images.push(p);
        }

        #[cfg(feature = "custom-primitive")]
        Primitive::Custom(p) => {
            if batch_entry.custom_primitives.len() < num_custom_pipelines {
                batch_entry
                    .custom_primitives
                    .resize(num_custom_pipelines, Vec::new());
            }

            let Some(custom_batch) = batch_entry
                .custom_primitives
                .get_mut(p.pipeline_index as usize)
            else {
                log::error!(
                    "Tried to add custom primitive with pipeline index {}, but canvas has {} custom pipelines, ignoring primitive",
                    p.pipeline_index,
                    num_custom_pipelines,
                );

                return;
            };

            custom_batch.push(QueuedCustomPrimitive {
                id: p.id,
                offset: p.offset,
            });
        }
    }
}

fn add_with_offset(
    primitive: impl Into<Primitive>,
    offset: Vector,
    batch_entry: &mut BatchEntry,
    #[cfg(feature = "custom-primitive")] num_custom_pipelines: usize,
) {
    let primitive: Primitive = primitive.into();

    match primitive {
        #[cfg(feature = "quad")]
        Primitive::SolidQuad(mut p) => {
            p.position[0] += offset.x;
            p.position[1] += offset.y;

            batch_entry.solid_quads.push(p);
        }
        #[cfg(all(feature = "quad", feature = "gradient"))]
        Primitive::GradientQuad(mut p) => {
            p.position[0] += offset.x;
            p.position[1] += offset.y;

            batch_entry.gradient_quads.push(p);
        }

        #[cfg(any(feature = "mesh", feature = "tessellation"))]
        Primitive::SolidMesh(mut p) => {
            p.uniform.offset[0] += offset.x;
            p.uniform.offset[1] += offset.y;

            batch_entry.solid_meshes.push(p);
        }
        #[cfg(all(any(feature = "mesh", feature = "tessellation"), feature = "gradient"))]
        Primitive::GradientMesh(mut p) => {
            p.uniform.offset[0] += offset.x;
            p.uniform.offset[1] += offset.y;

            batch_entry.gradient_meshes.push(p);
        }

        #[cfg(feature = "text")]
        Primitive::Text(mut p) => {
            p.pos.x += offset.x;
            p.pos.y += offset.y;

            batch_entry.text.push(p);
        }

        #[cfg(feature = "image")]
        Primitive::Image(mut p) => {
            p.vertex.position[0] += offset.x;
            p.vertex.position[1] += offset.y;

            batch_entry.images.push(p);
        }

        #[cfg(feature = "custom-primitive")]
        Primitive::Custom(p) => {
            if batch_entry.custom_primitives.len() < num_custom_pipelines {
                batch_entry
                    .custom_primitives
                    .resize(num_custom_pipelines, Vec::new());
            }

            let Some(custom_batch) = batch_entry
                .custom_primitives
                .get_mut(p.pipeline_index as usize)
            else {
                log::error!(
                    "Tried to add custom primitive with pipeline index {}, but canvas has {} custom pipelines, ignoring primitive",
                    p.pipeline_index,
                    num_custom_pipelines,
                );

                return;
            };

            custom_batch.push(QueuedCustomPrimitive {
                id: p.id,
                offset: Vector::new(p.offset.x + offset.x, p.offset.y + offset.y),
            });
        }
    }
}
