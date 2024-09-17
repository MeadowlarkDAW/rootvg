use std::num::NonZero;

use euclid::default::{Size2D, Transform2D, Vector2D};

use crate::{context::QueuedDrawingCommand, mesh::cache::MeshCache, Color, Vertex};

pub(crate) const INIT_VERTICES_CAPACITY: usize = 2048;
pub(crate) const INIT_ITEMS_CAPACITY: usize = 64;

pub(crate) struct Renderer {
    main_pipeline: Pipeline,

    pipeline_layout: wgpu::PipelineLayout,
    shader: wgpu::ShaderModule,

    vert_uniforms_buffer: wgpu::Buffer,
    vert_uniforms_bind_group: wgpu::BindGroup,

    item_uniforms_buffer: wgpu::Buffer,
    item_uniforms_bind_group: wgpu::BindGroup,
    item_uniforms_capacity: usize,

    vertex_buffer: wgpu::Buffer,
    vertex_buffer_capacity: usize,

    prev_view_size: Size2D<u32>,
}

impl Renderer {
    pub fn new(
        device: &wgpu::Device,
        texture_format: wgpu::TextureFormat,
        multisample: wgpu::MultisampleState,
    ) -> Self {
        let vert_uniforms_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("rootvg_vertex_uniforms_layout"),
                entries: &[VertUniforms::entry()],
            });

        let item_uniforms_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("rootvg_item_uniforms_layout"),
                entries: &[ItemUniforms::entry()],
            });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("rootvg_pipeline_layout"),
            bind_group_layouts: &[&vert_uniforms_layout, &item_uniforms_layout],
            push_constant_ranges: &[],
        });

        let vert_uniforms_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rootvg_vertex_uniforms_buffer"),
            size: std::mem::size_of::<VertUniforms>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let vert_uniforms_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rootvg_vertex_uniforms_bind_group"),
            layout: &vert_uniforms_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: vert_uniforms_buffer.as_entire_binding(),
            }],
        });

        let item_uniforms_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rootvg_item_uniforms_buffer"),
            size: (std::mem::size_of::<ItemUniforms>() * INIT_ITEMS_CAPACITY)
                as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let item_uniforms_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rootvg_item_uniforms_bind_group"),
            layout: &item_uniforms_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &item_uniforms_buffer,
                    offset: 0,
                    size: Some(NonZero::new(std::mem::size_of::<ItemUniforms>() as u64).unwrap()),
                }),
            }],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("rootvg_shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(concat!(
                include_str!("./shader/edge_aa.wgsl"),
                "\n",
                include_str!("./shader/shader.wgsl"),
            ))),
        });

        let main_pipeline = Pipeline::new(
            device,
            &pipeline_layout,
            &shader,
            texture_format,
            multisample,
        );

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rootvg_vertex_buffer"),
            size: (std::mem::size_of::<Vertex>() * INIT_VERTICES_CAPACITY) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            main_pipeline,

            pipeline_layout,
            shader,

            vert_uniforms_buffer,
            vert_uniforms_bind_group,

            item_uniforms_buffer,
            item_uniforms_bind_group,
            item_uniforms_capacity: INIT_ITEMS_CAPACITY,

            vertex_buffer,
            vertex_buffer_capacity: INIT_VERTICES_CAPACITY,

            prev_view_size: Size2D::zero(),
        }
    }

    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view_physical_size: Size2D<u32>,
        commands: &[QueuedDrawingCommand],
        mesh_cache: &MeshCache,
        total_num_vertices: usize,
    ) {
        if self.prev_view_size != view_physical_size {
            self.prev_view_size = view_physical_size;

            queue.write_buffer(
                &self.vert_uniforms_buffer,
                0,
                bytemuck::cast_slice(&[VertUniforms::new(view_physical_size)]),
            );
        }

        if commands.len() > self.item_uniforms_capacity {
            let new_size =
                (std::mem::size_of::<ItemUniforms>() * commands.len()).next_power_of_two();
            self.item_uniforms_capacity = new_size / std::mem::size_of::<ItemUniforms>();

            self.item_uniforms_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("rootvg_item_uniforms_buffer"),
                size: new_size as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }

        if total_num_vertices > self.vertex_buffer_capacity {
            let new_size = (std::mem::size_of::<Vertex>() * total_num_vertices).next_power_of_two();
            self.vertex_buffer_capacity = new_size / std::mem::size_of::<Vertex>();

            self.vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("rootvg_vertex_buffer"),
                size: new_size as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }

        for (i, cmd) in commands.iter().enumerate() {
            if cmd.copy_verts {
                let Some(mesh) = mesh_cache.get(cmd.mesh_id) else {
                    continue;
                };

                let vertices = if cmd.is_stroke {
                    &mesh.stroke_verts
                } else {
                    &mesh.fill_verts
                };

                if vertices.is_empty() {
                    continue;
                }

                let mut vertex_writer = queue
                    .write_buffer_with(
                        &self.vertex_buffer,
                        (cmd.vert_range.start as usize * std::mem::size_of::<Vertex>())
                            as wgpu::BufferAddress,
                        NonZero::new((vertices.len() * std::mem::size_of::<Vertex>()) as u64)
                            .unwrap(),
                    )
                    .unwrap();

                vertex_writer.copy_from_slice(bytemuck::cast_slice(vertices));
            }

            let mut item_writer = queue
                .write_buffer_with(
                    &self.item_uniforms_buffer,
                    (i * std::mem::size_of::<ItemUniforms>()) as wgpu::BufferAddress,
                    NonZero::new(std::mem::size_of::<ItemUniforms>() as u64).unwrap(),
                )
                .unwrap();

            item_writer.copy_from_slice(bytemuck::cast_slice(&[cmd.uniforms]));
        }
    }

    pub fn render<'pass>(
        &'pass self,
        render_pass: &mut wgpu::RenderPass<'pass>,
        commands: &[QueuedDrawingCommand],
    ) {
        render_pass.set_bind_group(0, &self.vert_uniforms_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_pipeline(&self.main_pipeline.pipeline);

        for (i, cmd) in commands.iter().enumerate() {
            render_pass.set_bind_group(
                1,
                &self.item_uniforms_bind_group,
                &[(i * std::mem::size_of::<ItemUniforms>()) as u32],
            );

            render_pass.draw(cmd.vert_range.clone(), 0..1);
        }
    }
}

struct Pipeline {
    pipeline: wgpu::RenderPipeline,
}

impl Pipeline {
    pub fn new(
        device: &wgpu::Device,
        pipeline_layout: &wgpu::PipelineLayout,
        shader: &wgpu::ShaderModule,
        texture_format: wgpu::TextureFormat,
        multisample: wgpu::MultisampleState,
    ) -> Self {
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("rootvg_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: texture_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions {
                    ..Default::default()
                },
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                //cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: None,
            multisample,
            multiview: None,
            cache: None,
        });

        Self { pipeline }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct VertUniforms {
    /// The reciprical of the view size times two
    pub view_size_recip_2: [f32; 2],
}

impl VertUniforms {
    fn new(view_physical_size: Size2D<u32>) -> Self {
        Self {
            view_size_recip_2: [
                2.0 / view_physical_size.width as f32,
                2.0 / view_physical_size.height as f32,
            ],
        }
    }

    fn entry() -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(
                    std::mem::size_of::<Self>() as wgpu::BufferAddress
                ),
            },
            count: None,
        }
    }
}

// TODO: An option to use push constants?
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct ItemUniforms {
    pub scissor_mat: [f32; 12],
    pub paint_mat: [f32; 12],
    pub inner_color: Color,
    pub outer_color: Color,
    pub scissor_ext: [f32; 2],
    pub scissor_scale: [f32; 2],
    pub extent: [f32; 2],
    pub offset: [f32; 2],
    pub radius: f32,
    pub feather: f32,
    pub stroke_mult: f32,
    pub stroke_thr: f32,
    pub text_type: u32,
    pub type_: u32,
    pub has_paint_mat: u32,
    /// Dynamic offset uniforms must be 256-aligned;
    /// see: [`wgpu::Limits`] `min_uniform_buffer_offset_alignment`.
    pub _padding: [u32; 17],
}

impl ItemUniforms {
    pub fn new(
        // arrays
        scissor_mat: &Transform2D<f32>,
        paint_mat: &Option<Transform2D<f32>>,
        inner_color: Color,
        outer_color: Color,
        scissor_ext: Size2D<f32>,
        scissor_scale: Vector2D<f32>,
        extent: Size2D<f32>,
        offset: Vector2D<f32>,
        radius: f32,
        feather: f32,
        stroke_mult: f32,
        stroke_thr: f32,
        text_type: u32,
        type_: ShaderType,
    ) -> Self {
        let (paint_mat, has_paint_mat) = if let Some(paint_mat) = paint_mat {
            (xform_to_mat3x4(&paint_mat.inverse().unwrap()), 1)
        } else {
            ([0.0; 12], 0)
        };

        Self {
            scissor_mat: xform_to_mat3x4(scissor_mat),
            paint_mat,
            inner_color,
            outer_color,
            scissor_ext: scissor_ext.into(),
            scissor_scale: scissor_scale.into(),
            extent: extent.into(),
            offset: offset.into(),
            radius,
            feather,
            stroke_mult,
            stroke_thr,
            text_type,
            type_: type_ as u32,
            has_paint_mat,
            // TODO: There could be a small performance gain here if these
            // padding bytes are left unitialized.
            _padding: [0; 17],
        }
    }

    pub fn entry() -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size: wgpu::BufferSize::new(
                    std::mem::size_of::<Self>() as wgpu::BufferAddress
                ),
            },
            count: None,
        }
    }
}

const fn xform_to_mat3x4(t: &Transform2D<f32>) -> [f32; 12] {
    [
        t.m11, t.m12, 0.0, 0.0, t.m21, t.m22, 0.0, 0.0, t.m31, t.m32, 1.0, 0.0,
    ]
}

#[repr(u32)]
pub(crate) enum ShaderType {
    Color = 0,
    Gradient,
    Image,
    Stencil,
    ImageGradient,
    FilterImage,
    TextureCopyUnclipped,
}
