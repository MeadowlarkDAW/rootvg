use rustc_hash::FxHashMap;

use crate::color::PackedSrgb;
use crate::error::RenderError;
use crate::math::{
    PhysicalSizeI32, PointI32, RectI32, ScaleFactor, Size, SizeI32, VectorI32, ZIndex,
};

#[cfg(feature = "msaa")]
use crate::msaa::MsaaPipeline;

#[cfg(all(any(feature = "mesh", feature = "tessellation"), feature = "gradient"))]
use crate::mesh::{
    pipeline::gradient::{GradientMeshBatchBuffer, GradientMeshPipeline},
    GradientMeshPrimitive,
};
#[cfg(any(feature = "mesh", feature = "tessellation"))]
use crate::mesh::{
    pipeline::solid::{SolidMeshBatchBuffer, SolidMeshPipeline},
    SolidMeshPrimitive,
};

#[cfg(all(feature = "quad", feature = "gradient"))]
use crate::quad::{
    pipeline::gradient::{GradientQuadBatchBuffer, GradientQuadPipeline},
    GradientQuadPrimitive,
};
#[cfg(feature = "quad")]
use crate::quad::{
    pipeline::solid::{SolidQuadBatchBuffer, SolidQuadPipeline},
    SolidQuadPrimitive,
};

#[cfg(feature = "text")]
use crate::text::{
    pipeline::{TextBatchBuffer, TextPipeline},
    FontSystem, TextPrimitive,
};

#[cfg(all(feature = "text", feature = "svg-icons"))]
use crate::text::svg::SvgIconSystem;

#[cfg(feature = "image")]
use rootvg_image::{
    pipeline::{ImageBatchBuffer, ImagePipeline},
    ImagePrimitive,
};

#[cfg(feature = "custom-primitive")]
use rootvg_core::pipeline::{
    CustomPipeline, CustomPipelineID, CustomPipelinePrimitive, CustomPrimitive,
};

mod context;

pub use context::CanvasCtx;

#[cfg(feature = "custom-primitive")]
struct CustomPipelineEntry {
    pipeline: Box<dyn CustomPipeline>,
    primitives_to_prepare: Vec<CustomPipelinePrimitive>,

    prev_prepared_screen_size: PhysicalSizeI32,
    prev_prepared_scale_factor: ScaleFactor,
    prev_prepared_primitives: Vec<CustomPipelinePrimitive>,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanvasConfig {
    pub multisample: wgpu::MultisampleState,
}

pub struct Canvas {
    batches: FxHashMap<BatchKey, BatchEntry>,
    temp_keys_for_sorting: Vec<BatchKey>,

    #[cfg(any(feature = "mesh", feature = "tessellation"))]
    solid_mesh_pipeline: SolidMeshPipeline,
    #[cfg(all(any(feature = "mesh", feature = "tessellation"), feature = "gradient"))]
    gradient_mesh_pipeline: GradientMeshPipeline,

    #[cfg(feature = "quad")]
    solid_quad_pipeline: SolidQuadPipeline,
    #[cfg(all(feature = "quad", feature = "gradient"))]
    gradient_quad_pipeline: GradientQuadPipeline,

    #[cfg(feature = "text")]
    text_pipeline: TextPipeline,

    #[cfg(feature = "image")]
    image_pipeline: ImagePipeline,

    #[cfg(feature = "msaa")]
    msaa_pipeline: Option<MsaaPipeline>,

    #[cfg(feature = "custom-primitive")]
    custom_pipelines: thunderdome::Arena<CustomPipelineEntry>,

    output: CanvasOutput,
    physical_size: PhysicalSizeI32,
    logical_size: Size,
    logical_size_i32: SizeI32,
    scale_factor: ScaleFactor,
    screen_to_clip_scale: [f32; 2],

    scissor_rect: RectI32,
    scissor_rect_out_of_bounds: bool,

    needs_preparing: bool,

    pub(crate) z_index: ZIndex,
}

impl Canvas {
    pub fn new(
        device: &wgpu::Device,
        #[allow(unused)] // queue is unused if the "text" feature is disabled
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        config: CanvasConfig,
        #[cfg(feature = "text")] font_system: &mut FontSystem,
    ) -> Self {
        let CanvasConfig { multisample } = config;

        Self {
            batches: FxHashMap::default(),
            temp_keys_for_sorting: Vec::new(),

            #[cfg(any(feature = "mesh", feature = "tessellation"))]
            solid_mesh_pipeline: SolidMeshPipeline::new(device, format, multisample),
            #[cfg(all(any(feature = "mesh", feature = "tessellation"), feature = "gradient"))]
            gradient_mesh_pipeline: GradientMeshPipeline::new(device, format, multisample),

            #[cfg(feature = "quad")]
            solid_quad_pipeline: SolidQuadPipeline::new(device, format, multisample),
            #[cfg(all(feature = "quad", feature = "gradient"))]
            gradient_quad_pipeline: GradientQuadPipeline::new(device, format, multisample),

            #[cfg(feature = "text")]
            text_pipeline: TextPipeline::new(device, queue, format, multisample, font_system),

            #[cfg(feature = "image")]
            image_pipeline: ImagePipeline::new(device, format, multisample),

            #[cfg(feature = "msaa")]
            msaa_pipeline: if multisample.count > 1 {
                Some(MsaaPipeline::new(device, format, multisample.count))
            } else {
                None
            },

            #[cfg(feature = "custom-primitive")]
            custom_pipelines: thunderdome::Arena::new(),

            output: CanvasOutput::new(),
            physical_size: PhysicalSizeI32::default(),
            logical_size: Size::default(),
            logical_size_i32: SizeI32::default(),
            scale_factor: ScaleFactor::default(),
            screen_to_clip_scale: [0.0; 2],
            scissor_rect: RectI32::default(),
            scissor_rect_out_of_bounds: true,
            needs_preparing: false,
            z_index: 0,
        }
    }

    pub fn begin(
        &mut self,
        physical_size: PhysicalSizeI32,
        scale_factor: ScaleFactor,
    ) -> CanvasCtx<'_> {
        assert!(physical_size.width > 0);
        assert!(physical_size.height > 0);
        assert!(scale_factor.0 > 0.0);

        // TODO: Try to re-use the allocated capacity of batch entries?
        self.batches.clear();

        self.scale_factor = scale_factor;
        self.physical_size = physical_size;
        self.logical_size = crate::math::to_logical_size_i32(physical_size, self.scale_factor);
        self.logical_size_i32 = SizeI32::new(
            self.logical_size.width.round() as i32,
            self.logical_size.height.round() as i32,
        );
        self.screen_to_clip_scale = [
            2.0 * scale_factor * (physical_size.width as f32).recip(),
            2.0 * scale_factor * (physical_size.height as f32).recip(),
        ];
        self.reset_scissor_rect();
        self.needs_preparing = true;
        self.z_index = 0;

        CanvasCtx { canvas: self }
    }

    pub fn set_scissor_rect(&mut self, scissor_rect: RectI32) {
        if self.scissor_rect != scissor_rect {
            if let Some(bounded_scissor_rect) =
                offset_scissor_rect(scissor_rect, VectorI32::new(0, 0), self.logical_size_i32)
            {
                self.scissor_rect = bounded_scissor_rect;
                self.scissor_rect_out_of_bounds = false;
            } else {
                self.scissor_rect = scissor_rect;
                self.scissor_rect_out_of_bounds = true;
            }
        }
    }

    pub fn reset_scissor_rect(&mut self) {
        self.scissor_rect = RectI32::new(PointI32::new(0, 0), self.logical_size_i32);
        self.scissor_rect_out_of_bounds = false;
    }

    #[cfg(feature = "custom-primitive")]
    pub fn insert_custom_pipeline(
        &mut self,
        pipeline: impl CustomPipeline + 'static,
    ) -> CustomPipelineID {
        CustomPipelineID(self.custom_pipelines.insert(CustomPipelineEntry {
            pipeline: Box::new(pipeline),
            primitives_to_prepare: Vec::new(),
            prev_prepared_screen_size: PhysicalSizeI32::default(),
            prev_prepared_scale_factor: ScaleFactor::default(),
            prev_prepared_primitives: Vec::new(),
        }))
    }

    #[cfg(feature = "custom-primitive")]
    pub fn custom_pipeline(&self, id: CustomPipelineID) -> Option<&Box<dyn CustomPipeline>> {
        self.custom_pipelines.get(id.0).map(|entry| &entry.pipeline)
    }

    #[cfg(feature = "custom-primitive")]
    pub fn custom_pipeline_mut(
        &mut self,
        id: CustomPipelineID,
    ) -> Option<&mut Box<dyn CustomPipeline>> {
        self.custom_pipelines
            .get_mut(id.0)
            .map(|entry| &mut entry.pipeline)
    }

    #[cfg(feature = "custom-primitive")]
    pub fn remove_custom_pipeline(
        &mut self,
        id: CustomPipelineID,
    ) -> Option<Box<dyn CustomPipeline>> {
        self.custom_pipelines
            .remove(id.0)
            .map(|entry| entry.pipeline)
    }

    pub fn render_to_target(
        &mut self,
        clear_color: Option<PackedSrgb>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        target_size: PhysicalSizeI32,
        #[cfg(feature = "text")] font_system: &mut FontSystem,
        #[cfg(all(feature = "text", feature = "svg-icons"))] svg_icon_system: &mut SvgIconSystem,
    ) -> Result<(), RenderError> {
        assert_eq!(target_size, self.physical_size);

        self.prepare(
            device,
            queue,
            #[cfg(feature = "text")]
            font_system,
            #[cfg(all(feature = "text", feature = "svg-icons"))]
            svg_icon_system,
        )?;

        let clear_color = clear_color.map(|c| wgpu::Color {
            r: c.r() as f64,
            g: c.g() as f64,
            b: c.b() as f64,
            a: c.a() as f64,
        });

        #[cfg(feature = "msaa")]
        let mut msaa_pipeline = self.msaa_pipeline.take();

        {
            #[cfg(feature = "msaa")]
            let (attachment, resolve_target, load) = if let Some(msaa_pipeline) = &mut msaa_pipeline
            {
                let (attachment, resolve_target) = msaa_pipeline.targets(device, target_size);

                (
                    attachment,
                    Some(resolve_target),
                    wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                )
            } else {
                (
                    target,
                    None,
                    if let Some(color) = clear_color {
                        wgpu::LoadOp::Clear(color)
                    } else {
                        wgpu::LoadOp::Load
                    },
                )
            };

            #[cfg(not(feature = "msaa"))]
            let (attachment, resolve_target, load) = (
                target,
                None,
                if let Some(color) = clear_color {
                    wgpu::LoadOp::Clear(color)
                } else {
                    wgpu::LoadOp::Load
                },
            );

            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("rootvg render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: attachment,
                    resolve_target,
                    ops: wgpu::Operations {
                        load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            self.render(&mut render_pass).unwrap();
        }

        #[cfg(feature = "msaa")]
        {
            // TODO: See if it's more performant to only use an MSAA render pass for pipelines
            // that need it (like the mesh pipelines). The tradeoff is that this will require
            // multiple render passes for each batch of meshes instead of just two render
            // passes for the whole canvas.
            if let Some(msaa_pipeline) = &mut msaa_pipeline {
                msaa_pipeline.render_to_target(target, clear_color, encoder);
            }

            self.msaa_pipeline = msaa_pipeline;
        }

        Ok(())
    }

    fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        #[cfg(feature = "text")] font_system: &mut FontSystem,
        #[cfg(all(feature = "text", feature = "svg-icons"))] svg_icon_system: &mut SvgIconSystem,
    ) -> Result<(), RenderError> {
        if !self.needs_preparing {
            return Ok(());
        }
        self.needs_preparing = false;

        #[cfg(feature = "quad")]
        self.solid_quad_pipeline.start_preparations(
            device,
            queue,
            self.physical_size,
            self.scale_factor,
        );
        #[cfg(all(feature = "quad", feature = "gradient"))]
        self.gradient_quad_pipeline.start_preparations(
            device,
            queue,
            self.physical_size,
            self.scale_factor,
        );

        #[cfg(any(feature = "mesh", feature = "tessellation"))]
        self.solid_mesh_pipeline.start_preparations(
            device,
            queue,
            self.physical_size,
            self.scale_factor,
        );
        #[cfg(all(any(feature = "mesh", feature = "tessellation"), feature = "gradient"))]
        self.gradient_mesh_pipeline.start_preparations(
            device,
            queue,
            self.physical_size,
            self.scale_factor,
        );

        #[cfg(feature = "text")]
        self.text_pipeline
            .start_preparations(device, queue, self.physical_size, self.scale_factor);

        #[cfg(feature = "image")]
        self.image_pipeline.start_preparations(
            device,
            queue,
            self.physical_size,
            self.scale_factor,
        );

        #[cfg(feature = "custom-primitive")]
        for (_, entry) in self.custom_pipelines.iter_mut() {
            entry.primitives_to_prepare.clear();
        }

        self.output.order.clear();

        // Sort the keys by z index
        self.temp_keys_for_sorting = self.batches.keys().copied().collect();
        self.temp_keys_for_sorting
            .sort_unstable_by(|a, b| a.z_index.cmp(&b.z_index));

        let mut current_scissor_rect = RectI32::default();

        #[cfg(feature = "quad")]
        let mut num_solid_quad_batches = 0;
        #[cfg(all(feature = "quad", feature = "gradient"))]
        let mut num_gradient_quad_batches = 0;

        #[cfg(any(feature = "mesh", feature = "tessellation"))]
        let mut num_solid_mesh_batches = 0;
        #[cfg(all(any(feature = "mesh", feature = "tessellation"), feature = "gradient"))]
        let mut num_gradient_mesh_batches = 0;

        #[cfg(feature = "text")]
        let mut num_text_batches = 0;

        #[cfg(feature = "image")]
        let mut num_image_batches = 0;

        #[cfg(feature = "custom-primitive")]
        let mut num_custom_batches = 0;

        for key in self.temp_keys_for_sorting.iter() {
            let Some(batch_entry) = self.batches.get(key) else {
                continue;
            };

            if key.scissor_rect != current_scissor_rect {
                current_scissor_rect = key.scissor_rect;

                self.output
                    .order
                    .push(BatchKind::ScissorRect(key.scissor_rect));
            };

            #[cfg(feature = "quad")]
            if !batch_entry.solid_quads.is_empty() {
                if num_solid_quad_batches == self.output.solid_quad_batches.len() {
                    self.output
                        .solid_quad_batches
                        .push(self.solid_quad_pipeline.create_batch(device));
                }

                self.solid_quad_pipeline.prepare_batch(
                    &mut self.output.solid_quad_batches[num_solid_quad_batches],
                    &batch_entry.solid_quads,
                    device,
                    queue,
                );

                self.output.order.push(BatchKind::SolidQuad {
                    batch_index: num_solid_quad_batches,
                });

                num_solid_quad_batches += 1;
            }

            #[cfg(all(feature = "quad", feature = "gradient"))]
            if !batch_entry.gradient_quads.is_empty() {
                if num_gradient_quad_batches == self.output.gradient_quad_batches.len() {
                    self.output
                        .gradient_quad_batches
                        .push(self.gradient_quad_pipeline.create_batch(device));
                }

                self.gradient_quad_pipeline.prepare_batch(
                    &mut self.output.gradient_quad_batches[num_gradient_quad_batches],
                    &batch_entry.gradient_quads,
                    device,
                    queue,
                );

                self.output.order.push(BatchKind::GradientQuad {
                    batch_index: num_gradient_quad_batches,
                });

                num_gradient_quad_batches += 1;
            }

            #[cfg(any(feature = "mesh", feature = "tessellation"))]
            if !batch_entry.solid_meshes.is_empty() {
                if num_solid_mesh_batches == self.output.solid_mesh_batches.len() {
                    self.output
                        .solid_mesh_batches
                        .push(self.solid_mesh_pipeline.create_batch(device));
                }

                self.solid_mesh_pipeline.prepare_batch(
                    &mut self.output.solid_mesh_batches[num_solid_mesh_batches],
                    &batch_entry.solid_meshes,
                    device,
                    queue,
                );

                self.output.order.push(BatchKind::SolidMesh {
                    batch_index: num_solid_mesh_batches,
                });

                num_solid_mesh_batches += 1;
            }

            #[cfg(all(any(feature = "mesh", feature = "tessellation"), feature = "gradient"))]
            if !batch_entry.gradient_meshes.is_empty() {
                if num_gradient_mesh_batches == self.output.gradient_mesh_batches.len() {
                    self.output
                        .gradient_mesh_batches
                        .push(self.gradient_mesh_pipeline.create_batch(device));
                }

                self.gradient_mesh_pipeline.prepare_batch(
                    &mut self.output.gradient_mesh_batches[num_gradient_mesh_batches],
                    &batch_entry.gradient_meshes,
                    device,
                    queue,
                );

                self.output.order.push(BatchKind::GradientMesh {
                    batch_index: num_gradient_mesh_batches,
                });

                num_gradient_mesh_batches += 1;
            }

            #[cfg(feature = "text")]
            if !batch_entry.text.is_empty() {
                if num_text_batches == self.output.text_batches.len() {
                    self.output
                        .text_batches
                        .push(self.text_pipeline.create_batch(device));
                }

                self.text_pipeline.prepare_batch(
                    &mut self.output.text_batches[num_text_batches],
                    &batch_entry.text,
                    device,
                    queue,
                    font_system,
                    #[cfg(feature = "svg-icons")]
                    svg_icon_system,
                )?;

                self.output.order.push(BatchKind::Text {
                    batch_index: num_text_batches,
                });

                num_text_batches += 1;
            }

            #[cfg(feature = "image")]
            if !batch_entry.images.is_empty() {
                if num_image_batches == self.output.image_batches.len() {
                    self.output
                        .image_batches
                        .push(self.image_pipeline.create_batch(device));
                }

                self.image_pipeline.prepare_batch(
                    &mut self.output.image_batches[num_image_batches],
                    &batch_entry.images,
                    device,
                    queue,
                );

                self.output.order.push(BatchKind::Image {
                    batch_index: num_image_batches,
                });

                num_image_batches += 1;
            }

            #[cfg(feature = "custom-primitive")]
            if !batch_entry.custom_primitives.is_empty() {
                for custom_primitive in batch_entry.custom_primitives.iter() {
                    let Some(pipeline) = self
                        .custom_pipelines
                        .get_mut(custom_primitive.pipeline_id.0)
                    else {
                        return Err(RenderError::InvalidCustomPipelineID(
                            custom_primitive.pipeline_id,
                        ));
                    };

                    if num_custom_batches == self.output.custom_batches.len() {
                        self.output.custom_batches.push(CustomBatchBuffer {
                            primitive_index: pipeline.primitives_to_prepare.len(),
                        });
                    } else {
                        self.output.custom_batches[num_custom_batches].primitive_index =
                            pipeline.primitives_to_prepare.len();
                    }

                    pipeline
                        .primitives_to_prepare
                        .push(CustomPipelinePrimitive {
                            primitive: std::rc::Rc::clone(&custom_primitive.primitive),
                            offset: custom_primitive.offset,
                        });

                    self.output.order.push(BatchKind::Custom {
                        pipeline_id: custom_primitive.pipeline_id,
                        batch_index: num_custom_batches,
                    });

                    num_custom_batches += 1;
                }
            }
        }

        // Prepare custom pipelines
        #[cfg(feature = "custom-primitive")]
        for (_, entry) in self.custom_pipelines.iter_mut() {
            if entry.primitives_to_prepare.is_empty() {
                continue;
            }

            if entry.prev_prepared_screen_size == self.physical_size
                && entry.prev_prepared_scale_factor == self.scale_factor
                && entry.prev_prepared_primitives == entry.primitives_to_prepare
            {
                continue;
            }

            entry.prev_prepared_screen_size = self.physical_size;
            entry.prev_prepared_scale_factor = self.scale_factor;
            entry.prev_prepared_primitives = entry.primitives_to_prepare.clone();

            if let Err(e) = entry.pipeline.prepare(
                device,
                queue,
                self.physical_size,
                self.scale_factor,
                &entry.primitives_to_prepare,
            ) {
                return Err(RenderError::CustomPipelinePrepareError(e));
            }
        }

        // Prune unused batches in output

        #[cfg(feature = "quad")]
        if num_solid_quad_batches < self.output.solid_quad_batches.len() {
            self.output
                .solid_quad_batches
                .resize_with(num_solid_quad_batches, || {
                    self.solid_quad_pipeline.create_batch(device)
                });
        }
        #[cfg(all(feature = "quad", feature = "gradient"))]
        if num_gradient_quad_batches < self.output.gradient_quad_batches.len() {
            self.output
                .gradient_quad_batches
                .resize_with(num_gradient_quad_batches, || {
                    self.gradient_quad_pipeline.create_batch(device)
                });
        }

        #[cfg(any(feature = "mesh", feature = "tessellation"))]
        if num_solid_mesh_batches < self.output.solid_mesh_batches.len() {
            self.output
                .solid_mesh_batches
                .resize_with(num_solid_mesh_batches, || {
                    self.solid_mesh_pipeline.create_batch(device)
                });
        }
        #[cfg(all(any(feature = "mesh", feature = "tessellation"), feature = "gradient"))]
        if num_gradient_mesh_batches < self.output.gradient_mesh_batches.len() {
            self.output
                .gradient_mesh_batches
                .resize_with(num_gradient_mesh_batches, || {
                    self.gradient_mesh_pipeline.create_batch(device)
                });
        }

        #[cfg(feature = "text")]
        {
            if num_text_batches < self.output.text_batches.len() {
                self.output
                    .text_batches
                    .resize_with(num_text_batches, || self.text_pipeline.create_batch(device));
            }

            self.text_pipeline.finish_preparations(device, queue);
        }

        #[cfg(feature = "image")]
        if num_image_batches < self.output.image_batches.len() {
            self.output
                .image_batches
                .resize_with(num_image_batches, || {
                    self.image_pipeline.create_batch(device)
                });
        }

        #[cfg(feature = "custom-primitive")]
        if num_custom_batches < self.output.custom_batches.len() {
            self.output
                .custom_batches
                .resize_with(num_custom_batches, || CustomBatchBuffer {
                    primitive_index: 0,
                });
        }

        Ok(())
    }

    fn render<'pass>(
        &'pass mut self,
        render_pass: &mut wgpu::RenderPass<'pass>,
    ) -> Result<(), RenderError> {
        let mut scissor_rect_in_bounds = true;

        for order in self.output.order.iter() {
            match order {
                #[cfg(feature = "quad")]
                BatchKind::SolidQuad { batch_index } => {
                    if !scissor_rect_in_bounds {
                        continue;
                    }

                    self.solid_quad_pipeline
                        .render_batch(&self.output.solid_quad_batches[*batch_index], render_pass);
                }
                #[cfg(all(feature = "quad", feature = "gradient"))]
                BatchKind::GradientQuad { batch_index } => {
                    if !scissor_rect_in_bounds {
                        continue;
                    }

                    self.gradient_quad_pipeline.render_batch(
                        &self.output.gradient_quad_batches[*batch_index],
                        render_pass,
                    );
                }
                #[cfg(feature = "text")]
                BatchKind::Text { batch_index } => {
                    if !scissor_rect_in_bounds {
                        continue;
                    }

                    self.text_pipeline
                        .render_batch(&self.output.text_batches[*batch_index], render_pass)?;
                }
                #[cfg(any(feature = "mesh", feature = "tessellation"))]
                BatchKind::SolidMesh { batch_index } => {
                    if !scissor_rect_in_bounds {
                        continue;
                    }

                    self.solid_mesh_pipeline
                        .render_batch(&self.output.solid_mesh_batches[*batch_index], render_pass);
                }
                #[cfg(all(any(feature = "mesh", feature = "tessellation"), feature = "gradient"))]
                BatchKind::GradientMesh { batch_index } => {
                    if !scissor_rect_in_bounds {
                        continue;
                    }

                    self.gradient_mesh_pipeline.render_batch(
                        &self.output.gradient_mesh_batches[*batch_index],
                        render_pass,
                    );
                }
                #[cfg(feature = "image")]
                BatchKind::Image { batch_index } => {
                    if !scissor_rect_in_bounds {
                        continue;
                    }

                    self.image_pipeline
                        .render_batch(&self.output.image_batches[*batch_index], render_pass);
                }
                #[cfg(feature = "custom-primitive")]
                BatchKind::Custom {
                    batch_index,
                    pipeline_id,
                } => {
                    if !scissor_rect_in_bounds {
                        continue;
                    }

                    let entry = self.custom_pipelines.get(pipeline_id.0).unwrap();

                    if let Err(e) = entry.pipeline.render_primitive(
                        self.output.custom_batches[*batch_index].primitive_index,
                        render_pass,
                    ) {
                        return Err(RenderError::CustomPipelineRenderError(e));
                    }
                }
                BatchKind::ScissorRect(scissor_rect) => {
                    let mut x = (scissor_rect.origin.x as f32 * self.scale_factor).round() as i32;
                    let mut y = (scissor_rect.origin.y as f32 * self.scale_factor).round() as i32;
                    let mut width =
                        (scissor_rect.size.width as f32 * self.scale_factor).round() as i32;
                    let mut height =
                        (scissor_rect.size.height as f32 * self.scale_factor).round() as i32;

                    if x + scissor_rect.size.width <= 0
                        || x >= self.physical_size.width
                        || y + scissor_rect.size.height <= 0
                        || y >= self.physical_size.height
                    {
                        // Scissor rect is off screen
                        scissor_rect_in_bounds = false;
                        continue;
                    }
                    scissor_rect_in_bounds = true;

                    // Scissor rect must be in bounds or wgpu will panic.
                    if x < 0 {
                        width += x;
                        x = 0;
                    }
                    if y < 0 {
                        height += y;
                        y = 0;
                    }
                    if x + width > self.physical_size.width {
                        width = self.physical_size.width - x;
                    }
                    if y + height > self.physical_size.height {
                        height = self.physical_size.height - y;
                    }

                    // TODO: There seems to be a bug in wgpu when setting scissoring rectangles
                    // with the OpenGL backend, causing rendering issues. Investigate this
                    // further.
                    //
                    // I have only tested the OpenGL backend on my Arch Linux system running on
                    // an AMD RX570 graphics card with the KDE desktop environment.
                    render_pass.set_scissor_rect(x as u32, y as u32, width as u32, height as u32);
                }
            }
        }

        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct BatchKey {
    scissor_rect: RectI32,
    z_index: u32,
}

impl BatchKey {
    fn new(scissor_rect: RectI32, main_z_index: ZIndex, inner_z_index: ZIndex) -> Self {
        Self {
            scissor_rect,
            z_index: (main_z_index as u32) << 16 | inner_z_index as u32,
        }
    }
}

struct BatchEntry {
    #[cfg(any(feature = "mesh", feature = "tessellation"))]
    solid_meshes: Vec<SolidMeshPrimitive>,
    #[cfg(all(any(feature = "mesh", feature = "tessellation"), feature = "gradient"))]
    gradient_meshes: Vec<GradientMeshPrimitive>,

    #[cfg(feature = "quad")]
    solid_quads: Vec<SolidQuadPrimitive>,
    #[cfg(all(feature = "quad", feature = "gradient"))]
    gradient_quads: Vec<GradientQuadPrimitive>,

    #[cfg(feature = "text")]
    text: Vec<TextPrimitive>,

    #[cfg(feature = "image")]
    images: Vec<ImagePrimitive>,

    #[cfg(feature = "custom-primitive")]
    custom_primitives: Vec<CustomPrimitive>,
}

impl BatchEntry {
    fn new() -> Self {
        Self {
            #[cfg(any(feature = "mesh", feature = "tessellation"))]
            solid_meshes: Vec::new(),
            #[cfg(all(any(feature = "mesh", feature = "tessellation"), feature = "gradient"))]
            gradient_meshes: Vec::new(),

            #[cfg(feature = "quad")]
            solid_quads: Vec::new(),
            #[cfg(all(feature = "quad", feature = "gradient"))]
            gradient_quads: Vec::new(),

            #[cfg(feature = "text")]
            text: Vec::new(),

            #[cfg(feature = "image")]
            images: Vec::new(),

            #[cfg(feature = "custom-primitive")]
            custom_primitives: Vec::new(),
        }
    }
}

struct CanvasOutput {
    #[cfg(any(feature = "mesh", feature = "tessellation"))]
    solid_mesh_batches: Vec<SolidMeshBatchBuffer>,
    #[cfg(all(any(feature = "mesh", feature = "tessellation"), feature = "gradient"))]
    gradient_mesh_batches: Vec<GradientMeshBatchBuffer>,

    #[cfg(feature = "quad")]
    solid_quad_batches: Vec<SolidQuadBatchBuffer>,
    #[cfg(all(feature = "quad", feature = "gradient"))]
    gradient_quad_batches: Vec<GradientQuadBatchBuffer>,

    #[cfg(feature = "text")]
    text_batches: Vec<TextBatchBuffer>,

    #[cfg(feature = "image")]
    image_batches: Vec<ImageBatchBuffer>,

    #[cfg(feature = "custom-primitive")]
    custom_batches: Vec<CustomBatchBuffer>,

    order: Vec<BatchKind>,
}

impl CanvasOutput {
    fn new() -> Self {
        Self {
            #[cfg(any(feature = "mesh", feature = "tessellation"))]
            solid_mesh_batches: Vec::new(),
            #[cfg(all(any(feature = "mesh", feature = "tessellation"), feature = "gradient"))]
            gradient_mesh_batches: Vec::new(),

            #[cfg(feature = "quad")]
            solid_quad_batches: Vec::new(),
            #[cfg(all(feature = "quad", feature = "gradient"))]
            gradient_quad_batches: Vec::new(),

            #[cfg(feature = "text")]
            text_batches: Vec::new(),

            #[cfg(feature = "image")]
            image_batches: Vec::new(),

            #[cfg(feature = "custom-primitive")]
            custom_batches: Vec::new(),

            order: Vec::new(),
        }
    }
}

enum BatchKind {
    #[cfg(feature = "quad")]
    SolidQuad {
        batch_index: usize,
    },
    #[cfg(all(feature = "quad", feature = "gradient"))]
    GradientQuad {
        batch_index: usize,
    },

    #[cfg(feature = "text")]
    Text {
        batch_index: usize,
    },

    #[cfg(any(feature = "mesh", feature = "tessellation"))]
    SolidMesh {
        batch_index: usize,
    },
    #[cfg(all(any(feature = "mesh", feature = "tessellation"), feature = "gradient"))]
    GradientMesh {
        batch_index: usize,
    },

    #[cfg(feature = "image")]
    Image {
        batch_index: usize,
    },

    #[cfg(feature = "custom-primitive")]
    Custom {
        pipeline_id: CustomPipelineID,
        batch_index: usize,
    },

    ScissorRect(RectI32),
}

#[cfg(feature = "custom-primitive")]
#[derive(Clone)]
struct CustomBatchBuffer {
    primitive_index: usize,
}

fn offset_scissor_rect(scissor_rect: RectI32, offset: VectorI32, size: SizeI32) -> Option<RectI32> {
    let x = scissor_rect.origin.x + offset.x;
    let y = scissor_rect.origin.y + offset.y;

    if x + scissor_rect.size.width <= 0
        || x >= size.width
        || y + scissor_rect.size.height <= 0
        || y >= size.height
    {
        // Scissor rect is off screen
        return None;
    }

    Some(RectI32::new(PointI32::new(x, y), scissor_rect.size))
}
