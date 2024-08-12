use std::error::Error;

use crate::math::{PhysicalSizeI32, ScaleFactor, Vector};

pub type PrimitiveID = u32;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CustomPrimitive {
    pub id: PrimitiveID,
    pub offset: Vector,
    pub pipeline_index: u8,
}

pub trait CustomPipeline {
    fn needs_preparing(&self) -> bool;

    fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        screen_size: PhysicalSizeI32,
        scale_factor: ScaleFactor,
        primitives: &[QueuedCustomPrimitive],
    ) -> Result<(), Box<dyn Error>>;

    fn render_primitives<'pass>(
        &'pass self,
        primitives: &[QueuedCustomPrimitive],
        render_pass: &mut wgpu::RenderPass<'pass>,
    ) -> Result<(), Box<dyn Error>>;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QueuedCustomPrimitive {
    pub id: PrimitiveID,
    pub offset: Vector,
}

/// A default shader uniform struct containing the scale factor and a scaling vector
/// used to convert from screen space to clip space.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct DefaultConstantUniforms {
    pub screen_to_clip_scale: [f32; 2],
    pub scale_factor: f32,
    pub _padding: f32,
}

impl DefaultConstantUniforms {
    pub fn new(screen_size: PhysicalSizeI32, scale_factor: ScaleFactor) -> Self {
        Self {
            screen_to_clip_scale: crate::math::screen_to_clip_scale(screen_size, scale_factor),
            scale_factor: scale_factor.0,
            _padding: 0.0,
        }
    }

    pub fn layout_buffer_and_bind_group(
        device: &wgpu::Device,
    ) -> (wgpu::BindGroupLayout, wgpu::Buffer, wgpu::BindGroup) {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("rootvg-core constants layout"),
            entries: &[Self::entry(0)],
        });

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rootvg-core constants buffer"),
            size: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rootvg-core constants bind group"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        (layout, buffer, bind_group)
    }

    pub fn prepare_buffer(
        buffer: &wgpu::Buffer,
        screen_size: PhysicalSizeI32,
        scale_factor: ScaleFactor,
        queue: &wgpu::Queue,
    ) {
        let uniforms = Self::new(screen_size, scale_factor);
        queue.write_buffer(buffer, 0, bytemuck::bytes_of(&uniforms));
    }

    pub fn entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
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
