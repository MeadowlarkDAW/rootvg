use std::ops::Range;

use rootvg_core::{
    buffer::Buffer,
    math::{PhysicalSizeI32, ScaleFactor},
    pipeline::DefaultConstantUniforms,
};
use wgpu::PipelineCompilationOptions;

use crate::{GradientMeshPrimitive, GradientVertex2D};

use super::{InstanceUniforms, INITIAL_INDEX_COUNT, INITIAL_VERTEX_COUNT};

struct Instance {
    range_in_vertex_buffer: Range<u32>,
    range_in_index_buffer: Range<u32>,
}

pub struct GradientMeshBatchBuffer {
    instances: Vec<Instance>,
    vertex_buffer: Buffer<GradientVertex2D>,
    index_buffer: Buffer<u32>,
    instance_uniforms_buffer: Buffer<InstanceUniforms>,
    instance_uniforms_bind_group: wgpu::BindGroup,
    temp_vertex_buffer: Vec<GradientVertex2D>,
    temp_index_buffer: Vec<u32>,
    temp_instance_uniforms_buffer: Vec<InstanceUniforms>,

    prev_primitives: Vec<GradientMeshPrimitive>,
}

impl GradientMeshBatchBuffer {
    pub fn new(device: &wgpu::Device, instance_uniforms_layout: &wgpu::BindGroupLayout) -> Self {
        let vertex_buffer = Buffer::new(
            device,
            "rootvg-mesh gradient vertex buffer",
            INITIAL_VERTEX_COUNT,
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        );

        let index_buffer = Buffer::new(
            device,
            "rootvg-mesh gradient index buffer",
            INITIAL_INDEX_COUNT,
            wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        );

        let instance_uniforms_buffer = Buffer::new(
            device,
            "rootvg-mesh gradient uniforms buffer",
            1,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        );

        let instance_uniforms_bind_group = Self::bind_group(
            device,
            &instance_uniforms_buffer.raw,
            instance_uniforms_layout,
        );

        Self {
            instances: Vec::new(),
            vertex_buffer,
            index_buffer,
            instance_uniforms_buffer,
            instance_uniforms_bind_group,
            temp_vertex_buffer: Vec::new(),
            temp_index_buffer: Vec::new(),
            temp_instance_uniforms_buffer: Vec::new(),
            prev_primitives: Vec::new(),
        }
    }

    fn bind_group(
        device: &wgpu::Device,
        buffer: &wgpu::Buffer,
        layout: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rootvg-mesh gradient uniforms bind group"),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer,
                    offset: 0,
                    size: InstanceUniforms::min_size(),
                }),
            }],
        })
    }

    pub fn prepare(
        &mut self,
        primitives: &[GradientMeshPrimitive],
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        instance_uniforms_layout: &wgpu::BindGroupLayout,
    ) {
        // Don't prepare if the list of primitives hasn't changed since the last
        // preparation.
        if primitives == self.prev_primitives {
            return;
        }
        self.prev_primitives = primitives.into();

        // TODO: Detect when multiple primitives share the same mesh and batch them
        // together into a separate draw call to reduce the amount of duplicated data
        // sent to the GPU? Testing is needed to see if this will actually improve
        // performance in practice.

        self.instances.clear();
        self.temp_index_buffer.clear();
        self.temp_vertex_buffer.clear();
        self.temp_instance_uniforms_buffer.clear();

        for mesh in primitives.iter() {
            let vertex_buffer_start = self.temp_vertex_buffer.len() as u32;
            let index_buffer_start = self.temp_index_buffer.len() as u32;

            self.temp_vertex_buffer
                .extend_from_slice(&mesh.mesh.buffers.vertices);
            self.temp_index_buffer
                .extend_from_slice(&mesh.mesh.buffers.indices);

            self.instances.push(Instance {
                range_in_vertex_buffer: vertex_buffer_start..self.temp_vertex_buffer.len() as u32,
                range_in_index_buffer: index_buffer_start..self.temp_index_buffer.len() as u32,
            });

            self.temp_instance_uniforms_buffer
                .push(InstanceUniforms::new(mesh.uniform));
        }

        let _ = self
            .vertex_buffer
            .expand_to_fit_new_size(device, self.temp_vertex_buffer.len());
        let _ = self
            .index_buffer
            .expand_to_fit_new_size(device, self.temp_index_buffer.len());

        let _ = self.vertex_buffer.write(queue, 0, &self.temp_vertex_buffer);
        let _ = self.index_buffer.write(queue, 0, &self.temp_index_buffer);

        if self
            .instance_uniforms_buffer
            .expand_to_fit_new_size(device, self.instances.len())
        {
            self.instance_uniforms_bind_group = Self::bind_group(
                device,
                &self.instance_uniforms_buffer.raw,
                instance_uniforms_layout,
            );
        }

        let _ = self
            .instance_uniforms_buffer
            .write(queue, 0, &self.temp_instance_uniforms_buffer);
    }
}

pub struct GradientMeshPipeline {
    pipeline: wgpu::RenderPipeline,

    constants_buffer: wgpu::Buffer,
    constants_bind_group: wgpu::BindGroup,
    instance_uniforms_layout: wgpu::BindGroupLayout,

    screen_size: PhysicalSizeI32,
    scale_factor: ScaleFactor,
}

impl GradientMeshPipeline {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        multisample: wgpu::MultisampleState,
    ) -> Self {
        let (constants_layout, constants_buffer, constants_bind_group) =
            DefaultConstantUniforms::layout_buffer_and_bind_group(device);

        let instance_uniforms_layout = super::instance_uniforms_layout(device);

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("rootvg-mesh gradient pipeline layout"),
            bind_group_layouts: &[&constants_layout, &instance_uniforms_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("rootvg-mesh gradient shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(concat!(
                include_str!("../shader/mesh.wgsl"),
                "\n",
                include_str!("../shader/gradient.wgsl"),
                "\n",
                include_str!("../shader/oklab.wgsl"),
            ))),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("rootvg-mesh gradient pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "gradient_vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GradientVertex2D>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array!(
                        // Position
                        0 => Float32x2,
                        // Colors 1-2
                        1 => Uint32x4,
                        // Colors 3-4
                        2 => Uint32x4,
                        // Offsets
                        3 => Uint32x2,
                        // Direction
                        4 => Float32x4
                    ),
                }],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "gradient_fs_main",
                targets: &super::color_target_state(format),
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
            cache: None,
        });

        Self {
            pipeline,
            constants_buffer,
            constants_bind_group,
            instance_uniforms_layout,
            screen_size: PhysicalSizeI32::default(),
            scale_factor: ScaleFactor::default(),
        }
    }

    pub fn create_batch(&mut self, device: &wgpu::Device) -> GradientMeshBatchBuffer {
        GradientMeshBatchBuffer::new(device, &self.instance_uniforms_layout)
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
        batch: &mut GradientMeshBatchBuffer,
        primitives: &[GradientMeshPrimitive],
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        batch.prepare(primitives, device, queue, &self.instance_uniforms_layout);
    }

    pub fn render_batch<'pass>(
        &'pass self,
        batch: &'pass GradientMeshBatchBuffer,
        render_pass: &mut wgpu::RenderPass<'pass>,
    ) {
        if batch.instances.is_empty() {
            return;
        }

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.constants_bind_group, &[]);

        let vertex_end = batch.instances.last().unwrap().range_in_vertex_buffer.end;
        let index_end = batch.instances.last().unwrap().range_in_index_buffer.end;

        render_pass.set_vertex_buffer(0, batch.vertex_buffer.slice(0..vertex_end as usize));
        render_pass.set_index_buffer(
            batch.index_buffer.slice(0..index_end as usize),
            wgpu::IndexFormat::Uint32,
        );

        for (i, instance) in batch.instances.iter().enumerate() {
            render_pass.set_bind_group(
                1,
                &batch.instance_uniforms_bind_group,
                &[(i * std::mem::size_of::<InstanceUniforms>()) as u32],
            );

            render_pass.draw_indexed(
                instance.range_in_index_buffer.start..instance.range_in_index_buffer.end,
                instance.range_in_vertex_buffer.start as i32,
                0..1,
            );
        }
    }
}
