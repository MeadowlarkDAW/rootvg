use std::cell::Ref;

use glyphon::{
    Cache, FontSystem, Resolution, SwashCache, TextArea, TextAtlas, TextRenderer, Viewport,
};

use rootvg_core::math::{PhysicalSizeI32, ScaleFactor};

use crate::primitive::TextPrimitive;

pub struct TextBatchBuffer {
    text_renderer: TextRenderer,
    prev_primitives: Vec<TextPrimitive>,
}

pub struct TextPipeline {
    swash_cache: SwashCache,
    //cache: Cache,
    atlas: TextAtlas,
    viewport: Viewport,
    multisample: wgpu::MultisampleState,
    screen_size: PhysicalSizeI32,
    scale_factor: ScaleFactor,
    prepare_all_batches: bool,
    atlas_needs_trimmed: bool,
}

impl TextPipeline {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        multisample: wgpu::MultisampleState,
    ) -> Self {
        let swash_cache = SwashCache::new();
        let cache = Cache::new(device);
        let viewport = Viewport::new(device, &cache);
        let atlas =
            TextAtlas::with_color_mode(device, queue, &cache, format, glyphon::ColorMode::Accurate);

        Self {
            swash_cache,
            //cache,
            atlas,
            viewport,
            multisample,
            screen_size: PhysicalSizeI32::default(),
            scale_factor: ScaleFactor::default(),
            prepare_all_batches: true,
            atlas_needs_trimmed: false,
        }
    }

    pub fn create_batch(&mut self, device: &wgpu::Device) -> TextBatchBuffer {
        TextBatchBuffer {
            text_renderer: TextRenderer::new(&mut self.atlas, device, self.multisample, None),
            prev_primitives: Vec::new(),
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
        self.prepare_all_batches = true;

        self.viewport.update(
            queue,
            Resolution {
                width: screen_size.width as u32,
                height: screen_size.height as u32,
            },
        );
    }

    pub fn prepare_batch(
        &mut self,
        batch: &mut TextBatchBuffer,
        primitives: &[TextPrimitive],
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        font_system: &mut FontSystem,
    ) -> Result<(), glyphon::PrepareError> {
        // Don't prepare if the list of primitives hasn't changed since the last
        // preparation.
        let primitives_are_the_same = primitives == batch.prev_primitives;
        if primitives_are_the_same && !self.prepare_all_batches {
            return Ok(());
        }

        if !primitives_are_the_same {
            batch.prev_primitives = primitives.into();
        }

        self.atlas_needs_trimmed = true;

        // TODO: Reuse the allocation of these Vecs?
        let borrowed_buffers: Vec<Ref<'_, glyphon::Buffer>> =
            primitives.iter().map(|p| p.buffer.raw_buffer()).collect();

        let text_areas: Vec<TextArea<'_>> = primitives
            .iter()
            .zip(borrowed_buffers.iter())
            .map(|(p, b)| TextArea {
                buffer: &*b,
                left: (p.pos.x * self.scale_factor).round(),
                top: (p.pos.y * self.scale_factor).round() + 0.5,
                scale: self.scale_factor.0,
                bounds: glyphon::TextBounds {
                    left: ((p.pos.x + p.clipping_bounds.min_x()) * self.scale_factor).round()
                        as i32,
                    top: ((p.pos.y + p.clipping_bounds.min_y()) * self.scale_factor).round() as i32,
                    right: ((p.pos.x + p.clipping_bounds.min_x() + p.clipping_bounds.width())
                        * self.scale_factor)
                        .round() as i32,
                    bottom: ((p.pos.y + p.clipping_bounds.min_y() + p.clipping_bounds.height())
                        * self.scale_factor)
                        .round() as i32,
                },
                default_color: glyphon::Color::rgba(p.color.r, p.color.g, p.color.b, p.color.a),
            })
            .collect();

        batch.text_renderer.prepare(
            device,
            queue,
            font_system,
            &mut self.atlas,
            &self.viewport,
            text_areas,
            &mut self.swash_cache,
        )
    }

    pub fn finish_preparations(&mut self, _device: &wgpu::Device, _queue: &wgpu::Queue) {
        self.prepare_all_batches = false;

        if !self.atlas_needs_trimmed {
            return;
        }
        self.atlas_needs_trimmed = false;

        self.atlas.trim();
    }

    pub fn render_batch<'pass>(
        &'pass self,
        batch: &'pass TextBatchBuffer,
        render_pass: &mut wgpu::RenderPass<'pass>,
    ) -> Result<(), glyphon::RenderError> {
        batch
            .text_renderer
            .render(&self.atlas, &self.viewport, render_pass)
    }
}
