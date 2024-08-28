use std::any::Any;
use std::error::Error;
use std::rc::Rc;

use crate::math::{PhysicalSizeI32, ScaleFactor, Vector};

pub type PrimitiveID = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CustomPipelineID(pub thunderdome::Index);

#[derive(Debug, Clone)]
pub struct CustomPrimitive {
    pub primitive: Rc<dyn Any>,
    pub offset: Vector,
    pub pipeline_id: CustomPipelineID,
}

impl CustomPrimitive {
    pub fn new(primitive: impl Any, pipeline_id: CustomPipelineID) -> Self {
        Self {
            primitive: Rc::new(primitive),
            offset: Vector::default(),
            pipeline_id,
        }
    }

    pub fn new_with_offset(
        primitive: impl Any,
        offset: Vector,
        pipeline_id: CustomPipelineID,
    ) -> Self {
        Self {
            primitive: Rc::new(primitive),
            offset,
            pipeline_id,
        }
    }

    pub fn new_from_rc(primitive: &Rc<dyn Any>, pipeline_id: CustomPipelineID) -> Self {
        Self {
            primitive: Rc::clone(primitive),
            offset: Vector::default(),
            pipeline_id,
        }
    }
}

impl PartialEq for CustomPrimitive {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.primitive, &other.primitive)
            && self.offset == other.offset
            && self.pipeline_id == other.pipeline_id
    }
}

pub trait CustomPipeline: Any {
    /// Prepare to render the given list of primitives
    ///
    /// Note, if the screen size, scale factor, and list of primitives have not
    /// changed since the last preparation, then Yarrow will automatically
    /// skip calling this method.
    fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        screen_size: PhysicalSizeI32,
        scale_factor: ScaleFactor,
        primitives: &[CustomPipelinePrimitive],
    ) -> Result<(), Box<dyn Error>>;

    /// Render a primitive
    ///
    /// The `primitive_index` is the index into the slice of primitives that
    /// was previously passed into `CustomPipeline::prepare`.
    fn render_primitive<'pass>(
        &'pass self,
        primitive_index: usize,
        render_pass: &mut wgpu::RenderPass<'pass>,
    ) -> Result<(), Box<dyn Error>>;
}

#[derive(Debug, Clone)]
pub struct CustomPipelinePrimitive {
    pub primitive: Rc<dyn Any>,
    pub offset: Vector,
}

impl PartialEq for CustomPipelinePrimitive {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.primitive, &other.primitive) && self.offset == other.offset
    }
}

/// A default shader uniform struct containing the scale factor and a scaling vector
/// used to convert from screen space to clip space.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct DefaultConstantUniforms {
    /// The reciprical of the screen size.
    pub screen_size_recip: [f32; 2],
    pub scale_factor: f32,
    pub _padding: f32,
}

impl DefaultConstantUniforms {
    pub fn new(screen_size: PhysicalSizeI32, scale_factor: ScaleFactor) -> Self {
        Self {
            screen_size_recip: [
                2.0 * (screen_size.width as f32).recip(),
                2.0 * (screen_size.height as f32).recip(),
            ],
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
