use std::cell::RefCell;
use std::rc::Rc;

use image::RgbaImage;
use rootvg_core::math::PhysicalSizeU32;

#[derive(Debug)]
enum TextureSource {
    Image {
        data_to_upload: Option<RgbaImage>,
        uploaded_texture: Option<wgpu::Texture>,
    },
    Prepass {
        view: wgpu::TextureView,
    },
}

#[derive(Debug)]
pub(crate) struct TextureInner {
    source: TextureSource,
    pub(crate) bind_group: Option<wgpu::BindGroup>,
}

#[derive(Debug)]
pub enum TextureReplaceError {
    DifferentSize,
    DifferentSourceType,
}

/// A source of raw image data.
///
/// Once this texture has been uploaded to the GPU, the image
/// data will be automatically removed from RAM.
#[derive(Debug)]
pub struct RcTexture {
    pub(crate) inner: Rc<RefCell<TextureInner>>,
    size: PhysicalSizeU32,
    generation: u64,
}

impl RcTexture {
    pub fn new(image: impl Into<RgbaImage>) -> Self {
        let image: RgbaImage = image.into();

        let dimensions = image.dimensions();

        Self {
            inner: Rc::new(RefCell::new(TextureInner {
                source: TextureSource::Image {
                    data_to_upload: Some(image),
                    uploaded_texture: None,
                },
                bind_group: None,
            })),
            size: PhysicalSizeU32::new(dimensions.0, dimensions.1),
            generation: 0,
        }
    }

    pub fn from_prepass_texture(texture_view: wgpu::TextureView, size: PhysicalSizeU32) -> Self {
        Self {
            inner: Rc::new(RefCell::new(TextureInner {
                source: TextureSource::Prepass { view: texture_view },
                bind_group: None,
            })),
            size,
            generation: 0,
        }
    }

    pub fn replace_with_image(
        &mut self,
        image: impl Into<RgbaImage>,
    ) -> Result<(), TextureReplaceError> {
        let image: RgbaImage = image.into();
        let dimensions = image.dimensions();
        let size = PhysicalSizeU32::new(dimensions.0, dimensions.1);

        if size != self.size {
            return Err(TextureReplaceError::DifferentSize);
        }

        let mut inner = RefCell::borrow_mut(&self.inner);

        let TextureSource::Image { data_to_upload, .. } = &mut inner.source else {
            return Err(TextureReplaceError::DifferentSourceType);
        };

        *data_to_upload = Some(image);

        self.generation += 1;

        Ok(())
    }

    pub fn replace_prepass_texture(
        &mut self,
        texture_view: wgpu::TextureView,
        size: PhysicalSizeU32,
    ) -> Result<(), TextureReplaceError> {
        if self.size != size {
            return Err(TextureReplaceError::DifferentSize);
        }

        let mut inner = RefCell::borrow_mut(&self.inner);

        let TextureSource::Prepass { view } = &mut inner.source else {
            return Err(TextureReplaceError::DifferentSourceType);
        };

        *view = texture_view;

        inner.bind_group = None;

        self.generation += 1;

        Ok(())
    }

    pub fn mark_prepass_texture_dirty(&mut self) {
        self.generation += 1;
    }

    pub fn size(&self) -> PhysicalSizeU32 {
        self.size
    }

    pub(crate) fn upload_if_needed(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
    ) {
        let mut inner = RefCell::borrow_mut(&self.inner);

        let TextureInner { source, bind_group } = &mut *inner;

        match source {
            TextureSource::Image {
                data_to_upload,
                uploaded_texture,
            } => {
                let Some(data_to_upload) = data_to_upload.take() else {
                    return;
                };

                if bind_group.is_none() {
                    let dimensions = data_to_upload.dimensions();
                    let texture_size = wgpu::Extent3d {
                        width: dimensions.0,
                        height: dimensions.1,
                        depth_or_array_layers: 1,
                    };

                    let texture = device.create_texture(&wgpu::TextureDescriptor {
                        // All textures are stored as 3D, we represent our 2D texture
                        // by setting depth to 1.
                        size: texture_size,
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: rootvg_core::color::SRGBA8_TEXTURE_FORMAT,
                        // TEXTURE_BINDING tells wgpu that we want to use this texture in shaders
                        // COPY_DST means that we want to copy data to this texture
                        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                        label: None,
                        view_formats: &[],
                    });

                    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

                    let new_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        layout: texture_bind_group_layout,
                        entries: &[wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&view),
                        }],
                        label: None,
                    });

                    *bind_group = Some(new_bind_group);

                    *uploaded_texture = Some(texture);
                };

                let uploaded_texture = uploaded_texture.as_ref().unwrap();

                let texture_size = wgpu::Extent3d {
                    width: self.size.width,
                    height: self.size.height,
                    depth_or_array_layers: 1,
                };

                queue.write_texture(
                    wgpu::ImageCopyTexture {
                        texture: uploaded_texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    &data_to_upload,
                    // The layout of the texture
                    wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(4 * self.size.width),
                        rows_per_image: Some(self.size.height),
                    },
                    texture_size,
                );
            }
            TextureSource::Prepass { view } => {
                if bind_group.is_some() {
                    return;
                }

                let new_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: texture_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(view),
                    }],
                    label: None,
                });

                *bind_group = Some(new_bind_group);
            }
        }
    }
}

impl Clone for RcTexture {
    fn clone(&self) -> Self {
        Self {
            inner: Rc::clone(&self.inner),
            size: self.size,
            generation: self.generation,
        }
    }
}

impl PartialEq for RcTexture {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner) && self.generation == other.generation
    }
}

impl From<RgbaImage> for RcTexture {
    fn from(image: RgbaImage) -> Self {
        RcTexture::new(image)
    }
}
