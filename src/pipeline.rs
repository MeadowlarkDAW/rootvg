use euclid::default::{Size2D, Transform2D};
use wgpu::util::DeviceExt;

use crate::{Color, Vertex};

pub struct Renderer {
    main_pipeline: Pipeline,

    pipeline_layout: wgpu::PipelineLayout,
    shader: wgpu::ShaderModule,
    vert_uniforms_buffer: wgpu::Buffer,
    vert_uniforms_bind_group: wgpu::BindGroup,
    frag_uniforms_buffer: wgpu::Buffer,
    frag_uniforms_bind_group: wgpu::BindGroup,

    prev_view_size: Size2D<u32>,
}

impl Renderer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_format: wgpu::TextureFormat,
        multisample: wgpu::MultisampleState,
    ) -> Self {
        let vert_uniforms_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("rootvg_vertex_uniforms_layout"),
                entries: &[VertUniforms::entry()],
            });

        let frag_uniforms_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("rootvg_fragment_uniforms_layout"),
                entries: &[FragUniforms::entry()],
            });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("rootvg_pipeline_layout"),
            bind_group_layouts: &[&vert_uniforms_layout, &frag_uniforms_layout],
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

        let frag_uniforms_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rootvg_fragment_uniforms_buffer"),
            size: std::mem::size_of::<FragUniforms>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let frag_uniforms_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rootvg_fragment_uniforms_bind_group"),
            layout: &frag_uniforms_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: frag_uniforms_buffer.as_entire_binding(),
            }],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("rootvg_shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "shader.wgsl"
            ))),
        });

        let main_pipeline = Pipeline::new(
            device,
            &pipeline_layout,
            &shader,
            texture_format,
            multisample,
        );

        Self {
            main_pipeline,

            pipeline_layout,
            shader,
            vert_uniforms_buffer,
            vert_uniforms_bind_group,
            frag_uniforms_buffer,
            frag_uniforms_bind_group,

            prev_view_size: Size2D::zero(),
        }
    }

    pub fn prepare(&mut self, view_physical_size: Size2D<u32>, queue: &wgpu::Queue) {
        if self.prev_view_size != view_physical_size {
            self.prev_view_size = view_physical_size;

            queue.write_buffer(
                &self.vert_uniforms_buffer,
                0,
                bytemuck::cast_slice(&[VertUniforms::new(view_physical_size)]),
            );
        }

        self.main_pipeline
            .prepare(queue, &self.frag_uniforms_buffer);
    }

    pub fn render<'pass>(&'pass self, render_pass: &mut wgpu::RenderPass<'pass>) {
        render_pass.set_bind_group(0, &self.vert_uniforms_bind_group, &[]);

        self.main_pipeline.render(render_pass);
    }
}

struct Pipeline {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
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
                    //constants: &[("edge_aa".to_owned(), false)].into(),
                    ..Default::default()
                },
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Cw,
                ..Default::default()
            },
            depth_stencil: None,
            multisample,
            multiview: None,
            cache: None,
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices()),
            usage: wgpu::BufferUsages::VERTEX,
        });

        Self {
            pipeline,
            vertex_buffer,
        }
    }

    pub fn prepare(&mut self, queue: &wgpu::Queue, frag_uniforms_buffer: &wgpu::Buffer) {
        // test buffer
        queue.write_buffer(
            frag_uniforms_buffer,
            0,
            bytemuck::cast_slice(&[FragUniforms {
                scissor_mat: Transform2D::identity().into(),
                paint_mat: [f32; 12],
                inner_color: Color,
                outer_color: Color,
                scissor_ext: [f32; 2],
                scissor_scale: [f32; 2],
                extent: [f32; 2],
                radius: f32,
                feather: f32,
                stroke_mult: f32,
                stroke_thr: f32,
                text_type: u32,
                type_: u32,
            }]),
        );
    }

    pub fn render<'pass>(&'pass self, render_pass: &mut wgpu::RenderPass<'pass>) {
        render_pass.set_pipeline(&self.pipeline);

        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..3, 0..1);
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
struct FragUniforms {
    scissor_mat: [f32; 12],
    paint_mat: [f32; 12],
    inner_color: Color,
    outer_color: Color,
    scissor_ext: [f32; 2],
    scissor_scale: [f32; 2],
    extent: [f32; 2],
    radius: f32,
    feather: f32,
    stroke_mult: f32,
    stroke_thr: f32,
    text_type: u32,
    type_: u32,
    /*
    /// Dynamic offset uniforms must be 256-aligned;
    /// see: [`wgpu::Limits`] `min_uniform_buffer_offset_alignment`.
    _padding: [f32; 20],
    */
}

impl FragUniforms {
    pub fn entry() -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
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

fn vertices() -> Vec<Vertex> {
    vec![
        Vertex {
            pos: [0.0, 0.5].into(),
            uv: [0.0, 0.0].into(),
        },
        Vertex {
            pos: [-0.5, -0.5].into(),
            uv: [0.0, 0.0].into(),
        },
        Vertex {
            pos: [0.5, -0.5].into(),
            uv: [0.0, 0.0].into(),
        },
    ]
}
