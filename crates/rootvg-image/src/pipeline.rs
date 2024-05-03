use rustc_hash::FxHashMap;
use smallvec::SmallVec;
use std::{cell::RefCell, ops::Range, rc::Rc};
use wgpu::PipelineCompilationOptions;

use rootvg_core::{
    buffer::Buffer,
    math::{PhysicalSizeI32, ScaleFactor},
    pipeline::DefaultConstantUniforms,
};

use crate::{
    primitive::{ImagePrimitive, ImageVertex},
    texture::TextureInner,
    RcTexture,
};

const INITIAL_INSTANCES: usize = 16;
const INITIAL_SUB_BATCHES: usize = 16;

struct Batch {
    range_in_buffer: Range<u32>,
    texture: RcTexture,
}

pub struct ImageBatchBuffer {
    buffer: Buffer<ImageVertex>,
    sub_batches: Vec<Batch>,
    num_instances: usize,

    prev_primitives: Vec<ImagePrimitive>,
}

impl ImageBatchBuffer {
    fn new(device: &wgpu::Device) -> Self {
        Self {
            buffer: Buffer::new(
                device,
                "rootvg-image instance buffer",
                INITIAL_INSTANCES,
                wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            ),
            sub_batches: Vec::with_capacity(INITIAL_SUB_BATCHES),
            num_instances: 0,
            prev_primitives: Vec::new(),
        }
    }

    fn prepare(
        &mut self,
        primitives: &[ImagePrimitive],
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
    ) {
        // Don't prepare if primitives have not changed since the last
        // prepare.
        if primitives == &self.prev_primitives {
            return;
        }
        self.prev_primitives = primitives.into();

        self.sub_batches.clear();
        self.num_instances = primitives.len();

        self.buffer.expand_to_fit_new_size(device, primitives.len());

        struct TempSubBatchEntry {
            vertices: SmallVec<[ImageVertex; INITIAL_INSTANCES]>,
            texture: RcTexture,
        }

        // TODO: reuse the allocation of this hash map?
        let mut sub_batches_map: FxHashMap<*const RefCell<TextureInner>, TempSubBatchEntry> =
            FxHashMap::default();
        sub_batches_map.reserve(INITIAL_SUB_BATCHES);

        for image in primitives.iter() {
            image
                .texture
                .upload_if_needed(device, queue, texture_bind_group_layout);

            let sub_batch = sub_batches_map
                .entry(Rc::as_ptr(&image.texture.inner))
                .or_insert_with(|| TempSubBatchEntry {
                    vertices: SmallVec::new(),
                    texture: image.texture.clone(),
                });

            sub_batch.vertices.push(image.vertex);
        }

        let mut range_start = 0;
        for sub_batch in sub_batches_map.values() {
            self.buffer.write(queue, range_start, &sub_batch.vertices);

            self.sub_batches.push(Batch {
                range_in_buffer: range_start as u32
                    ..(range_start + sub_batch.vertices.len()) as u32,
                texture: sub_batch.texture.clone(),
            });

            range_start += sub_batch.vertices.len();
        }
    }
}

pub struct ImagePipeline {
    pipeline: wgpu::RenderPipeline,

    constants_buffer: wgpu::Buffer,
    constants_bind_group: wgpu::BindGroup,
    texture_layout: wgpu::BindGroupLayout,

    screen_size: PhysicalSizeI32,
    scale_factor: ScaleFactor,
}

impl ImagePipeline {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        multisample: wgpu::MultisampleState,
    ) -> Self {
        let constants_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("rootvg-image constants layout"),
            entries: &[
                DefaultConstantUniforms::entry(0),
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    // This should match the filterable field of the
                    // Texture entry.
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let constants_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rootvg-image constants buffer"),
            size: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let constants_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rootvg-image constants bind group"),
            layout: &constants_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: constants_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("rootvg-image texture layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            }],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("rootvg-image pipeline layout"),
            push_constant_ranges: &[],
            bind_group_layouts: &[&constants_layout, &texture_layout],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("rootvg-image shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(concat!(include_str!(
                "shader/image.wgsl"
            ),))),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("rootvg-image pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<ImageVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array!(
                        // Position
                        0 => Float32x2,
                        // Size
                        1 => Float32x2,
                        // Normalized UV position
                        2 => Float32x2,
                        // Normalized UV size
                        3 => Float32x2,
                        // Transform Matrix 3x2
                        4 => Float32x2,
                        5 => Float32x2,
                        6 => Float32x2,
                        // Has Transformation
                        7 => Uint32,
                    ),
                }],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Cw,
                ..Default::default()
            },
            depth_stencil: None,
            multisample,
            multiview: None,
        });

        Self {
            pipeline,
            constants_buffer,
            constants_bind_group,
            texture_layout,
            screen_size: PhysicalSizeI32::default(),
            scale_factor: ScaleFactor::default(),
        }
    }

    pub fn create_batch(&mut self, device: &wgpu::Device) -> ImageBatchBuffer {
        ImageBatchBuffer::new(device)
    }

    pub fn start_preparations(
        &mut self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        screen_size: PhysicalSizeI32,
        scale_factor: ScaleFactor,
    ) {
        if self.screen_size == screen_size && self.scale_factor == scale_factor {
            return;
        }

        self.screen_size = screen_size;
        self.scale_factor = scale_factor;

        DefaultConstantUniforms::prepare_buffer(
            &self.constants_buffer,
            screen_size,
            scale_factor,
            queue,
        );
    }

    pub fn prepare_batch(
        &mut self,
        batch: &mut ImageBatchBuffer,
        primitives: &[ImagePrimitive],
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        batch.prepare(primitives, device, queue, &self.texture_layout);
    }

    pub fn render_batch<'pass>(
        &'pass self,
        batch: &'pass ImageBatchBuffer,
        render_pass: &mut wgpu::RenderPass<'pass>,
    ) {
        if batch.num_instances == 0 {
            return;
        }

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.constants_bind_group, &[]);

        render_pass.set_vertex_buffer(0, batch.buffer.slice(0..batch.num_instances));

        for sub_batch in batch.sub_batches.iter() {
            // # SAFETY:
            //
            // Because wgpu requires the bind group to be borrowed for `'pass`, we
            // are not able to use the safe option that returns a `std::cell::Ref`.
            //
            // By design, data is only mutated during the prepare stage, not during
            // the render pass stage. So there is no chance for the
            // `RefCell<TextureInner>` to be borrowed mutably during the render
            // pass.
            let texture_bind_group = unsafe {
                &RefCell::try_borrow_unguarded(&sub_batch.texture.inner)
                    .unwrap()
                    .bind_group
                    .as_ref()
                    .unwrap()
            };

            render_pass.set_bind_group(1, texture_bind_group, &[]);

            render_pass.draw(0..6, sub_batch.range_in_buffer.clone());
        }
    }
}
