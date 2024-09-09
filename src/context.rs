use std::ops::Range;

use euclid::default::{Point2D, Size2D, Transform2D, Vector2D};

use crate::{
    mesh::{
        cache::{MeshCache, MeshCacheEntry},
        tessellator::Tessellator,
        MeshBuilder, MeshID,
    },
    paint::PaintType,
    pipeline::{ItemUniforms, Renderer, ShaderType, INIT_ITEMS_CAPACITY},
    Paint, Vertex,
};

struct ScissorUniforms {
    mat: Transform2D<f32>,
    ext: Size2D<f32>,
    scale: Vector2D<f32>,
}

pub struct Context {
    mesh_cache: MeshCache,
    mesh_builder: MeshBuilder,
    tessellator: Tessellator,
    renderer: Renderer,

    scissor_mat: Transform2D<f32>,
    scissor_ext: Size2D<f32>,
    scissor_uniforms: Option<ScissorUniforms>,
    global_alpha: f32,
    view_physical_size: Size2D<u32>,
    scale_factor: f32,
    antialias: bool,

    queued_items: Vec<QueuedItem>,
    total_num_vertices: usize,
    needs_preparing: bool,
}

impl Context {
    pub fn new(
        device: &wgpu::Device,
        texture_format: wgpu::TextureFormat,
        multisample: wgpu::MultisampleState,
        scale_factor: f32,
        antialias: bool,
    ) -> Self {
        assert!(scale_factor > 0.0);

        let mut mesh_builder = MeshBuilder::new();
        let mut tessellator = Tessellator::new();

        mesh_builder.set_scale_factor(scale_factor);
        tessellator.set_scale_factor(scale_factor);

        Self {
            mesh_cache: MeshCache::new(),
            mesh_builder,
            tessellator,
            renderer: Renderer::new(device, texture_format, multisample),

            scissor_mat: Transform2D::identity(),
            scissor_ext: Size2D::default(),
            scissor_uniforms: None,
            global_alpha: 1.0,
            view_physical_size: Size2D::default(),
            scale_factor,
            antialias,

            queued_items: Vec::with_capacity(INIT_ITEMS_CAPACITY),
            total_num_vertices: 0,
            needs_preparing: false,
        }
    }

    pub fn begin_frame<'a>(
        &'a mut self,
        view_physical_size: Size2D<u32>,
        scale_factor: f32,
    ) -> ContextRef<'a> {
        assert!(view_physical_size.width > 0);
        assert!(view_physical_size.height > 0);
        assert!(scale_factor > 0.0);

        let force_mesh_rebuild = if self.scale_factor != scale_factor {
            self.scale_factor = scale_factor;

            self.mesh_builder.set_scale_factor(scale_factor);
            self.tessellator.set_scale_factor(scale_factor);

            true
        } else {
            false
        };

        self.mesh_cache.begin_frame();
        self.queued_items.clear();
        self.total_num_vertices = 0;
        self.needs_preparing = true;
        self.view_physical_size = view_physical_size;

        ContextRef {
            r: self,
            force_mesh_rebuild,
        }
    }

    fn build_mesh_cached(
        &mut self,
        f: impl FnOnce(&mut MeshBuilder),
        force_rebuild: bool,
    ) -> MeshID {
        self.mesh_builder.clear();

        (f)(&mut self.mesh_builder);

        self.mesh_cache.build_mesh_cached(
            &mut self.tessellator,
            &mut self.mesh_builder,
            self.antialias,
            force_rebuild,
        )
    }

    fn build_mesh_uncached(
        &mut self,
        f: impl FnOnce(&mut MeshBuilder),
        prev_mesh_id: &mut Option<MeshID>,
    ) -> MeshID {
        self.mesh_builder.clear();

        (f)(&mut self.mesh_builder);

        self.mesh_cache.build_mesh_uncached(
            &mut self.tessellator,
            &mut self.mesh_builder,
            self.antialias,
            prev_mesh_id,
        )
    }

    fn fill_mesh_with_offset(&mut self, mesh_id: MeshID, paint: &Paint, offset: Vector2D<f32>) {
        let Some(mesh) = self.mesh_cache.get_mut(mesh_id) else {
            return;
        };

        let (vert_range, copy_verts) = if let Some(vert_range) = mesh.fill_vert_range.clone() {
            (vert_range, false)
        } else {
            if mesh.fill_verts.is_empty() {
                return;
            }

            let vert_range =
                self.total_num_vertices..self.total_num_vertices + mesh.fill_verts.len();
            mesh.fill_vert_range = Some(vert_range.clone());

            self.total_num_vertices += mesh.fill_verts.len();

            (vert_range, true)
        };

        self.queue_mesh(mesh_id, vert_range, paint, offset, false, 0.0, copy_verts);
    }

    fn stroke_mesh_with_offset(&mut self, mesh_id: MeshID, paint: &Paint, offset: Vector2D<f32>) {
        let Some(mesh) = self.mesh_cache.get_mut(mesh_id) else {
            return;
        };

        let (vert_range, copy_verts) = if let Some(vert_range) = mesh.stroke_vert_range.clone() {
            (vert_range, false)
        } else {
            if mesh.stroke_verts.is_empty() {
                return;
            }

            let vert_range =
                self.total_num_vertices..self.total_num_vertices + mesh.stroke_verts.len();
            mesh.stroke_vert_range = Some(vert_range.clone());

            self.total_num_vertices += mesh.stroke_verts.len();

            (vert_range, true)
        };

        let stroke_width = mesh.stroke_width;
        self.queue_mesh(
            mesh_id,
            vert_range,
            paint,
            offset,
            true,
            stroke_width,
            copy_verts,
        );
    }

    fn queue_mesh(
        &mut self,
        mesh_id: MeshID,
        vert_range: Range<usize>,
        paint: &Paint,
        offset: Vector2D<f32>,
        is_stroke: bool,
        mut stroke_width: f32,
        copy_verts: bool,
    ) {
        let fringe_width = self.tessellator.fringe_width();

        let (stroke_mult, stroke_thr, alpha_mult) = if is_stroke {
            let alpha_mult = if stroke_width < fringe_width {
                // If the stroke width is less than pixel size, use alpha to emulate coverage.
                // Since coverage is area, scale by alpha*alpha.
                let alpha = (stroke_width * self.scale_factor).clamp(0.0, 1.0);
                stroke_width = fringe_width;

                alpha * alpha
            } else {
                1.0
            };

            let stroke_mult = (stroke_width * 0.5 + fringe_width * 0.5) * self.scale_factor;

            let stroke_thr = 1.0 - 0.5 / 255.0;

            (stroke_mult, stroke_thr, alpha_mult)
        } else {
            (1.0, -1.0, 1.0)
        };

        let alpha_mult = alpha_mult * self.global_alpha;

        let scissor_uniforms = if let Some(s) = &self.scissor_uniforms {
            s
        } else {
            self.scissor_uniforms = Some(
                if self.scissor_ext.width < -0.5 || self.scissor_ext.height < -0.5 {
                    ScissorUniforms {
                        mat: Transform2D::identity(),
                        ext: Size2D::new(1.0, 1.0),
                        scale: Vector2D::new(1.0, 1.0),
                    }
                } else {
                    ScissorUniforms {
                        mat: self.scissor_mat.inverse().unwrap(),
                        ext: self.scissor_ext,
                        scale: Vector2D::new(
                            (self.scissor_mat.m11 * self.scissor_mat.m11
                                + self.scissor_mat.m21 * self.scissor_mat.m21)
                                .sqrt()
                                * self.scale_factor,
                            (self.scissor_mat.m12 * self.scissor_mat.m12
                                + self.scissor_mat.m22 * self.scissor_mat.m22)
                                .sqrt()
                                * self.scale_factor,
                        ),
                    }
                },
            );

            self.scissor_uniforms.as_ref().unwrap()
        };

        let uniforms = match paint.paint_type {
            PaintType::SolidColor(mut color) => {
                color.a *= alpha_mult;

                // Premultiply color
                color.r *= color.a;
                color.g *= color.a;
                color.b *= color.a;

                ItemUniforms::new(
                    &scissor_uniforms.mat,
                    &paint.transform,
                    color,
                    crate::color::TRANSPARENT,
                    scissor_uniforms.ext,
                    scissor_uniforms.scale,
                    Size2D::default(),
                    offset,
                    0.0,
                    0.0,
                    stroke_mult,
                    stroke_thr,
                    0,
                    ShaderType::Color,
                )
            }
            PaintType::Gradient {
                inner_color,
                outer_color,
                extent,
                radius,
                feather,
            } => {
                todo!()
            }
            PaintType::Image { image_id, extent } => {
                todo!()
            }
        };

        self.queued_items.push(QueuedItem {
            mesh_id,
            is_stroke,
            vert_range,
            uniforms,
            copy_verts,
        });
    }

    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if !self.needs_preparing {
            return;
        }
        self.needs_preparing = false;

        self.renderer.prepare(
            device,
            queue,
            self.view_physical_size,
            &self.queued_items,
            &self.mesh_cache,
            self.total_num_vertices,
        );
    }

    pub fn render<'pass>(&'pass self, render_pass: &mut wgpu::RenderPass<'pass>) {
        self.renderer.render(render_pass, &self.queued_items);
    }
}

pub struct ContextRef<'a> {
    r: &'a mut Context,
    force_mesh_rebuild: bool,
}

impl<'a> ContextRef<'a> {
    pub fn build_mesh(&mut self, f: impl FnOnce(&mut MeshBuilder)) -> MeshID {
        self.r.build_mesh_cached(f, self.force_mesh_rebuild)
    }

    pub fn build_mesh_uncached(
        &mut self,
        f: impl FnOnce(&mut MeshBuilder),
        prev_mesh_id: &mut Option<MeshID>,
    ) -> MeshID {
        self.r.build_mesh_uncached(f, prev_mesh_id)
    }

    pub fn fill_mesh(&mut self, mesh_id: MeshID, paint: &Paint) {
        self.r
            .fill_mesh_with_offset(mesh_id, paint, Vector2D::zero());
    }

    pub fn stroke_mesh(&mut self, mesh_id: MeshID, paint: &Paint) {
        self.r
            .stroke_mesh_with_offset(mesh_id, paint, Vector2D::zero());
    }

    pub fn fill_mesh_with_offset(&mut self, mesh_id: MeshID, paint: &Paint, offset: Vector2D<f32>) {
        self.r.fill_mesh_with_offset(mesh_id, paint, offset);
    }

    pub fn stroke_mesh_with_offset(
        &mut self,
        mesh_id: MeshID,
        paint: &Paint,
        offset: Vector2D<f32>,
    ) {
        self.r.stroke_mesh_with_offset(mesh_id, paint, offset);
    }

    pub fn set_global_alpha(&mut self, alpha: f32) {
        self.r.global_alpha = alpha.clamp(0.0, 1.0);
    }

    pub fn global_alpha(&self) -> f32 {
        self.r.global_alpha
    }
}

pub(crate) struct QueuedItem {
    pub mesh_id: MeshID,
    pub is_stroke: bool,
    pub vert_range: Range<usize>,
    pub uniforms: ItemUniforms,
    pub copy_verts: bool,
}
