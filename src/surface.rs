use wgpu::MultisampleState;

use crate::{
    canvas::CanvasConfig,
    math::{PhysicalSizeI32, ScaleFactor},
};

#[derive(Debug)]
pub struct DefaultSurfaceConfig {
    pub present_mode: wgpu::PresentMode,
    pub power_preference: wgpu::PowerPreference,
    pub instance_descriptor: wgpu::InstanceDescriptor,
    pub force_fallback_adapter: bool,
    pub limits: Option<wgpu::Limits>,
    pub desired_maximum_frame_latency: u32,
    pub memory_hints: wgpu::MemoryHints,

    #[cfg(feature = "msaa")]
    pub antialiasing: Option<rootvg_msaa::Antialiasing>,
}

impl Clone for DefaultSurfaceConfig {
    fn clone(&self) -> Self {
        Self {
            present_mode: self.present_mode,
            power_preference: self.power_preference,
            // `wgpu::InstanceDescriptor` doesn't implement `Clone` for some reason
            instance_descriptor: wgpu::InstanceDescriptor {
                backends: self.instance_descriptor.backends,
                flags: self.instance_descriptor.flags,
                dx12_shader_compiler: self.instance_descriptor.dx12_shader_compiler.clone(),
                gles_minor_version: self.instance_descriptor.gles_minor_version,
            },
            force_fallback_adapter: self.force_fallback_adapter,
            limits: self.limits.clone(),
            desired_maximum_frame_latency: self.desired_maximum_frame_latency,
            memory_hints: self.memory_hints.clone(),

            #[cfg(feature = "msaa")]
            antialiasing: self.antialiasing,
        }
    }
}

impl Default for DefaultSurfaceConfig {
    fn default() -> Self {
        Self {
            present_mode: wgpu::PresentMode::AutoVsync,
            power_preference: wgpu::PowerPreference::None,
            // The instance is a handle to our GPU
            // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
            instance_descriptor: wgpu::InstanceDescriptor {
                backends: wgpu::Backends::all(),
                ..Default::default()
            },
            force_fallback_adapter: false,
            limits: None,
            desired_maximum_frame_latency: 2,
            memory_hints: wgpu::MemoryHints::default(),

            #[cfg(feature = "msaa")]
            antialiasing: Some(rootvg_msaa::Antialiasing::MSAAx8),
        }
    }
}

struct SurfaceConfigInner {
    present_mode: wgpu::PresentMode,
    power_preference: wgpu::PowerPreference,
    force_fallback_adapter: bool,
    limits: Option<wgpu::Limits>,
    desired_maximum_frame_latency: u32,
    memory_hints: wgpu::MemoryHints,

    #[cfg(feature = "msaa")]
    antialiasing: Option<rootvg_msaa::Antialiasing>,
}

/// The default wgpu surface handled by RootVG.
pub struct DefaultSurface<'a> {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'a>,
    pub surface_config: wgpu::SurfaceConfiguration,
    scale_factor: ScaleFactor,

    #[cfg(feature = "msaa")]
    largest_compatible_aa: Option<rootvg_msaa::Antialiasing>,
}

impl<'a> DefaultSurface<'a> {
    /// Create a new surface from the given window handle.
    ///
    /// - `size` - The size of the surface in physical pixels
    /// - `scale_factor` - The scale factor of the surface in pixels per point
    /// - `window` - A handle to the window. (Note, to give [`DefaultSurface`] a static
    /// lifetime, wrap the window handle inside of an `Arc`).
    /// - `config` - Additional settings for the surface
    pub fn new(
        physical_size: PhysicalSizeI32,
        scale_factor: ScaleFactor,
        window: impl Into<wgpu::SurfaceTarget<'a>>,
        config: DefaultSurfaceConfig,
    ) -> Result<Self, NewSurfaceError> {
        assert!(physical_size.width > 0);
        assert!(physical_size.height > 0);

        let DefaultSurfaceConfig {
            #[cfg(feature = "msaa")]
            antialiasing,
            present_mode,
            power_preference,
            instance_descriptor,
            force_fallback_adapter,
            limits,
            desired_maximum_frame_latency,
            memory_hints,
        } = config;

        let instance = wgpu::Instance::new(instance_descriptor);

        if log::max_level() > log::LevelFilter::Info {
            let available_adapters: Vec<_> = instance
                .enumerate_adapters(wgpu::Backends::all())
                .iter()
                .map(wgpu::Adapter::get_info)
                .collect();

            log::trace!("available wgpu adapters: {available_adapters:#?}");
        }

        let surface = instance.create_surface(window)?;

        pollster::block_on(Self::new_async(
            physical_size,
            scale_factor,
            instance,
            surface,
            SurfaceConfigInner {
                present_mode,
                power_preference,
                force_fallback_adapter,
                limits,
                desired_maximum_frame_latency,
                memory_hints,
                #[cfg(feature = "msaa")]
                antialiasing,
            },
        ))
    }

    /// Create a new surface from the given window handle.
    ///
    /// - `size` - The size of the surface in physical pixels
    /// - `scale_factor` - The scale factor of the surface in pixels per point
    /// - `window` - A handle to the window
    /// - `config` - Additional settings for the surface
    ///
    /// # Safety
    ///
    /// * `window` must outlive the resulting surface target (and subsequently the surface created for this target).
    pub unsafe fn new_unsafe(
        physical_size: PhysicalSizeI32,
        scale_factor: ScaleFactor,
        window: wgpu::SurfaceTargetUnsafe,
        config: DefaultSurfaceConfig,
    ) -> Result<Self, NewSurfaceError> {
        assert!(physical_size.width > 0);
        assert!(physical_size.height > 0);

        let DefaultSurfaceConfig {
            #[cfg(feature = "msaa")]
            antialiasing,
            present_mode,
            power_preference,
            instance_descriptor,
            force_fallback_adapter,
            limits,
            desired_maximum_frame_latency,
            memory_hints,
        } = config;

        let instance = wgpu::Instance::new(instance_descriptor);

        if log::max_level() > log::LevelFilter::Info {
            let available_adapters: Vec<_> = instance
                .enumerate_adapters(wgpu::Backends::all())
                .iter()
                .map(wgpu::Adapter::get_info)
                .collect();

            log::trace!("available wgpu adapters: {available_adapters:#?}");
        }

        let surface = instance.create_surface_unsafe(window)?;

        pollster::block_on(Self::new_async(
            physical_size,
            scale_factor,
            instance,
            surface,
            SurfaceConfigInner {
                present_mode,
                power_preference,
                force_fallback_adapter,
                limits,
                desired_maximum_frame_latency,
                memory_hints,
                #[cfg(feature = "msaa")]
                antialiasing,
            },
        ))
    }

    async fn new_async(
        physical_size: PhysicalSizeI32,
        scale_factor: ScaleFactor,
        instance: wgpu::Instance,
        surface: wgpu::Surface<'a>,
        config: SurfaceConfigInner,
    ) -> Result<Self, NewSurfaceError> {
        let SurfaceConfigInner {
            #[cfg(feature = "msaa")]
            antialiasing,
            present_mode,
            power_preference,
            force_fallback_adapter,
            limits,
            desired_maximum_frame_latency,
            memory_hints,
        } = config;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference,
                compatible_surface: Some(&surface),
                force_fallback_adapter,
            })
            .await
            .ok_or_else(|| NewSurfaceError::CouldNotGetAdapter)?;

        // WGPU already logs this at info level
        //log::info!("selected wgpu adapter: {:#?}", adapter.get_info());

        let (texture_format, alpha_mode) = {
            let capabilities = surface.get_capabilities(&adapter);

            let mut formats = capabilities.formats.iter().copied();

            log::trace!("available texture formats: {formats:#?}");

            // Gamma correction
            #[cfg(not(feature = "web-colors"))]
            let format = formats.find(wgpu::TextureFormat::is_srgb);

            // No gamma correction
            #[cfg(feature = "web-colors")]
            let format = formats.find(|format| !wgpu::TextureFormat::is_srgb(format));

            let format = format.or_else(|| {
                log::warn!("no texture format found!");

                capabilities.formats.first().copied()
            });

            let alpha_modes = capabilities.alpha_modes;

            log::trace!("available alpha modes: {alpha_modes:#?}");

            let preferred_alpha = if alpha_modes.contains(&wgpu::CompositeAlphaMode::PostMultiplied)
            {
                wgpu::CompositeAlphaMode::PostMultiplied
            } else {
                wgpu::CompositeAlphaMode::Auto
            };

            (
                format.ok_or_else(|| NewSurfaceError::NoCompatibleTextureFormat)?,
                preferred_alpha,
            )
        };

        log::info!(
            "selected wgpu texture format: {texture_format:?} with alpha mode: {alpha_mode:?}"
        );

        let limits_vec = if let Some(limits) = limits {
            vec![limits]
        } else {
            vec![wgpu::Limits::default(), wgpu::Limits::downlevel_defaults()]
        };

        let mut limits = limits_vec.clone().into_iter().map(|limits| wgpu::Limits {
            max_bind_groups: 2,
            ..limits
        });

        let mut required_features = wgpu::Features::empty();
        #[cfg(all(feature = "msaa", not(target_arch = "wasm32")))]
        if let Some(antialiasing) = antialiasing {
            // The WebGPU spec only gaurantees a sample count of 1 or 4
            if antialiasing != rootvg_msaa::Antialiasing::MSAAx4 {
                required_features.insert(wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES);
            }
        }

        let (device, queue) = loop {
            let required_limits = if let Some(r) = limits.next() {
                r
            } else {
                // If this feauture is not compatible, try again without it. This will limit
                // us to only being able to use `Antialiasing::MSAAx4`.
                if required_features
                    .contains(wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES)
                {
                    required_features = wgpu::Features::empty();

                    let mut limits = limits_vec.clone().into_iter().map(|limits| wgpu::Limits {
                        max_bind_groups: 2,
                        ..limits
                    });

                    limits.next().unwrap()
                } else {
                    return Err(NewSurfaceError::NoDeviceWithCompatibleLimits);
                }
            };

            let device = adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: Some("rootvg::renderer device descriptor"),
                        required_features,
                        required_limits,
                        memory_hints: memory_hints.clone(),
                    },
                    None,
                )
                .await
                .ok();

            if let Some(device_and_queue) = device {
                break device_and_queue;
            }
        };

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: texture_format,
            width: physical_size.width as u32,
            height: physical_size.height as u32,
            present_mode,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency,
        };
        surface.configure(&device, &surface_config);

        #[cfg(feature = "msaa")]
        let largest_compatible_aa = {
            let format_feature_flags = adapter.get_texture_format_features(texture_format).flags;
            let mut largest_compatible_aa = antialiasing;
            loop {
                match largest_compatible_aa {
                    Some(rootvg_msaa::Antialiasing::MSAAx16) => {
                        if format_feature_flags
                            .contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X16)
                        {
                            break;
                        }
                        largest_compatible_aa = Some(rootvg_msaa::Antialiasing::MSAAx8);
                    }
                    Some(rootvg_msaa::Antialiasing::MSAAx8) => {
                        if format_feature_flags
                            .contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X8)
                        {
                            break;
                        }
                        largest_compatible_aa = Some(rootvg_msaa::Antialiasing::MSAAx4);
                    }
                    Some(rootvg_msaa::Antialiasing::MSAAx4) => {
                        if format_feature_flags
                            .contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X4)
                        {
                            break;
                        }
                        largest_compatible_aa = Some(rootvg_msaa::Antialiasing::MSAAx2);
                    }
                    Some(rootvg_msaa::Antialiasing::MSAAx2) => {
                        if format_feature_flags
                            .contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X2)
                        {
                            break;
                        }
                        largest_compatible_aa = None;
                    }
                    None => break,
                }
            }

            log::info!(
                "requested AA mode: {:?} | largest compatible antialiasing mode: {:?}",
                antialiasing,
                largest_compatible_aa
            );

            largest_compatible_aa
        };

        Ok(Self {
            surface,
            device,
            queue,
            surface_config,
            scale_factor,

            #[cfg(feature = "msaa")]
            largest_compatible_aa,
        })
    }

    /// Resize the surface.
    ///
    /// # Panics
    /// - `size.width` or `size.height` is zero
    /// - An old `wgpu::SurfaceTexture` is still alive referencing an old surface.
    pub fn resize(&mut self, physical_size: PhysicalSizeI32, scale_factor: ScaleFactor) {
        assert!(physical_size.width > 0);
        assert!(physical_size.height > 0);

        if self.surface_config.width == physical_size.width as u32
            && self.surface_config.height == physical_size.height as u32
            && self.scale_factor == scale_factor
        {
            return;
        }

        self.surface_config.width = physical_size.width as u32;
        self.surface_config.height = physical_size.height as u32;
        self.scale_factor = scale_factor;

        self.surface.configure(&self.device, &self.surface_config);
    }

    pub fn get_current_texture(&self) -> Result<wgpu::SurfaceTexture, wgpu::SurfaceError> {
        self.surface.get_current_texture()
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.surface_config.format
    }

    pub fn canvas_config(&self) -> CanvasConfig {
        #[cfg(feature = "msaa")]
        let sample_count = self
            .largest_compatible_aa
            .map(|aa| aa.sample_count())
            .unwrap_or(1);

        #[cfg(not(feature = "msaa"))]
        let sample_count = 1;

        CanvasConfig {
            multisample: MultisampleState {
                count: sample_count,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[cfg(feature = "msaa")]
    pub fn largest_compatible_aa(&self) -> Option<rootvg_msaa::Antialiasing> {
        self.largest_compatible_aa
    }
}

#[cfg(feature = "default-surface")]
#[derive(thiserror::Error, Debug)]
pub enum NewSurfaceError {
    #[error("failed to create wgpu surface from window: {0}")]
    CouldNotCreateSurface(#[from] wgpu::CreateSurfaceError),
    #[error("failed to get compatible wgpu adapter")]
    CouldNotGetAdapter,
    #[error("could not find compatible wgpu texture format")]
    NoCompatibleTextureFormat,
    #[error("could not find wgpu device with compatible limits")]
    NoDeviceWithCompatibleLimits,
}
