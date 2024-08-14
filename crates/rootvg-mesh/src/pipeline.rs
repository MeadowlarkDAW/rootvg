//! Draw meshes of triangles.
pub mod solid;

#[cfg(feature = "gradient")]
pub mod gradient;

use crate::MeshUniforms;

const INITIAL_INDEX_COUNT: usize = 256;
const INITIAL_VERTEX_COUNT: usize = 256;

fn instance_uniforms_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("rootvg-mesh instance uniforms layout"),
        entries: &[InstanceUniforms::entry()],
    })
}

fn color_target_state(format: wgpu::TextureFormat) -> [Option<wgpu::ColorTargetState>; 1] {
    [Some(wgpu::ColorTargetState {
        format,
        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
        write_mask: wgpu::ColorWrites::ALL,
    })]
}

// TODO: Use push constants instead if it is available.
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
struct InstanceUniforms {
    inner: MeshUniforms,
    /// Uniform values must be 256-aligned;
    /// see: [`wgpu::Limits`] `min_uniform_buffer_offset_alignment`.
    _padding1: [f32; 32],
    /// Bytemuck doesn't derive for arrays of size 55, so split it up.
    _padding2: [f32; 22],
}

impl InstanceUniforms {
    pub fn new(inner: MeshUniforms) -> Self {
        // # SAFETY:
        //
        // Neither the rust code nor the shader code reads these padding bytes.
        #[allow(invalid_value, clippy::uninit_assumed_init)]
        let (_padding1, _padding2): ([f32; 32], [f32; 22]) = unsafe {
            (
                std::mem::MaybeUninit::uninit().assume_init(),
                std::mem::MaybeUninit::uninit().assume_init(),
            )
        };

        Self {
            inner,
            _padding1,
            _padding2,
        }
    }

    fn entry() -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<Self>() as u64),
            },
            count: None,
        }
    }

    pub fn min_size() -> Option<wgpu::BufferSize> {
        wgpu::BufferSize::new(std::mem::size_of::<Self>() as u64)
    }
}
