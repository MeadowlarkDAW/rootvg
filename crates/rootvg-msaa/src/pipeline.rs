// The following code was copied and modified from
// https://github.com/iced-rs/iced/blob/31d1d5fecbef50fa319cabd5d4194f1e4aaefa21/wgpu/src/triangle/msaa.rs
// Iced license (MIT): https://github.com/iced-rs/iced/blob/31d1d5fecbef50fa319cabd5d4194f1e4aaefa21/LICENSE

use rootvg_core::math::PhysicalSizeI32;

#[derive(Debug)]
pub struct MsaaPipeline {
    format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
    constants: wgpu::BindGroup,
    texture_layout: wgpu::BindGroupLayout,
    sample_count: u32,
    targets: Option<Targets>,
}

impl MsaaPipeline {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, sample_count: u32) -> Self {
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());

        let constant_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("rootvg-msaa uniforms layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                count: None,
            }],
        });

        let constant_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rootvg-msaa uniforms bind group"),
            layout: &constant_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Sampler(&sampler),
            }],
        });

        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("rootvg-msaa texture layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("rootvg-msaa pipeline layout"),
            push_constant_ranges: &[],
            bind_group_layouts: &[&constant_layout, &texture_layout],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("rootvg triangle blit_shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "shader/msaa.wgsl"
            ))),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("rootvg-msaa pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Cw,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        Self {
            format,
            pipeline,
            constants: constant_bind_group,
            texture_layout,
            sample_count,
            targets: None,
        }
    }

    pub fn targets(
        &mut self,
        device: &wgpu::Device,
        size: PhysicalSizeI32,
    ) -> (&wgpu::TextureView, &wgpu::TextureView) {
        match &mut self.targets {
            None => {
                self.targets = Some(Targets::new(
                    device,
                    self.format,
                    &self.texture_layout,
                    self.sample_count,
                    size,
                ));
            }
            Some(targets) => {
                if targets.size != size {
                    self.targets = Some(Targets::new(
                        device,
                        self.format,
                        &self.texture_layout,
                        self.sample_count,
                        size,
                    ));
                }
            }
        }

        let targets = self.targets.as_ref().unwrap();

        (&targets.attachment, &targets.resolve)
    }

    pub fn render_to_target(
        &self,
        target: &wgpu::TextureView,
        clear_color: Option<wgpu::Color>,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let load = if let Some(color) = clear_color {
            wgpu::LoadOp::Clear(color)
        } else {
            wgpu::LoadOp::Load
        };

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("rootvg-msaa render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.constants, &[]);
        render_pass.set_bind_group(1, &self.targets.as_ref().unwrap().bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }
}

#[derive(Debug)]
struct Targets {
    attachment: wgpu::TextureView,
    resolve: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    size: PhysicalSizeI32,
}

impl Targets {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        texture_layout: &wgpu::BindGroupLayout,
        sample_count: u32,
        size: PhysicalSizeI32,
    ) -> Targets {
        assert!(size.width > 0);
        assert!(size.height > 0);

        let extent = wgpu::Extent3d {
            width: size.width as u32,
            height: size.height as u32,
            depth_or_array_layers: 1,
        };

        let attachment = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("rootvg-msaa attachment"),
            size: extent,
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let resolve = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("rootvg-msaa resolve target"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let attachment = attachment.create_view(&wgpu::TextureViewDescriptor::default());

        let resolve = resolve.create_view(&wgpu::TextureViewDescriptor::default());

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rootvg-msaa texture bind group"),
            layout: texture_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&resolve),
            }],
        });

        Targets {
            attachment,
            resolve,
            bind_group,
            size,
        }
    }
}
