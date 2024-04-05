// The following code is taken and modified from
// https://github.com/iced-rs/iced/blob/master/wgpu/src/quad/gradient.rs
//
// Iced MIT license: https://github.com/iced-rs/iced/blob/master/LICENSE

use rootvg_core::{
    buffer::Buffer,
    math::{PhysicalSizeI32, ScaleFactor},
    pipeline::DefaultConstantUniforms,
};

use super::INITIAL_INSTANCES;

use crate::GradientQuadPrimitive;

pub struct GradientQuadBatchBuffer {
    buffer: Buffer<GradientQuadPrimitive>,
    num_primitives: usize,
}

pub struct GradientQuadPipeline {
    pipeline: wgpu::RenderPipeline,

    constants_buffer: wgpu::Buffer,
    constants_bind_group: wgpu::BindGroup,

    screen_size: PhysicalSizeI32,
    scale_factor: ScaleFactor,
}

impl GradientQuadPipeline {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        multisample: wgpu::MultisampleState,
    ) -> Self {
        let (constants_layout, constants_buffer, constants_bind_group) =
            DefaultConstantUniforms::layout_buffer_and_bind_group(device);

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("rootvg-quad gradient pipeline layout"),
            push_constant_ranges: &[],
            bind_group_layouts: &[&constants_layout],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("rootvg-quad gradient shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(concat!(
                include_str!("../shader/quad.wgsl"),
                "\n",
                include_str!("../shader/gradient.wgsl"),
                "\n",
                include_str!("../shader/oklab.wgsl")
            ))),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("rootvg-quad gradient pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "gradient_vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GradientQuadPrimitive>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array!(
                        // Colors 1-2
                        0 => Uint32x4,
                        // Colors 3-4
                        1 => Uint32x4,
                        // Colors 5-6
                        2 => Uint32x4,
                        // Colors 7-8
                        3 => Uint32x4,
                        // Offsets 1-8
                        4 => Uint32x4,
                        // Direction
                        5 => Float32x4,
                        // Position
                        6 => Float32x2,
                        // Size
                        7 => Float32x2,
                        // Border color
                        8 => Float32x4,
                        // Border radius
                        9 => Float32x4,
                        // Border width
                        10 => Float32
                    ),
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "gradient_fs_main",
                targets: &super::color_target_state(format),
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
            constants_buffer,
            constants_bind_group,
            pipeline,
            screen_size: PhysicalSizeI32::default(),
            scale_factor: ScaleFactor::default(),
        }
    }

    pub fn create_batch(&mut self, device: &wgpu::Device) -> GradientQuadBatchBuffer {
        GradientQuadBatchBuffer {
            buffer: Buffer::new(
                device,
                "rootvg-quad gradient buffer",
                INITIAL_INSTANCES,
                wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            ),
            num_primitives: 0,
        }
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
        batch: &mut GradientQuadBatchBuffer,
        primitives: &[GradientQuadPrimitive],
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        let _ = batch
            .buffer
            .expand_to_fit_new_size(device, primitives.len());
        let _ = batch.buffer.write(queue, 0, primitives);

        batch.num_primitives = primitives.len();
    }

    pub fn render_batch<'pass>(
        &'pass self,
        batch: &'pass GradientQuadBatchBuffer,
        render_pass: &mut wgpu::RenderPass<'pass>,
    ) {
        if batch.num_primitives == 0 {
            return;
        }

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.constants_bind_group, &[]);

        render_pass.set_vertex_buffer(0, batch.buffer.slice(0..batch.num_primitives));

        render_pass.draw(0..6, 0..batch.num_primitives as u32);
    }
}
