use std::ops::Range;

use euclid::{
    default::{Size2D, Transform2D, Vector2D},
    Angle,
};
use vec1::Vec1;

const INIT_STATE_CAPACITY: usize = 8;

use crate::{
    mesh::{
        builder::MeshBuilder,
        cache::{MeshCache, MeshCacheKey, MeshID, RawMeshID},
        tessellator::Tessellator,
    },
    pipeline::{ItemUniforms, Renderer, ShaderType, INIT_ITEMS_CAPACITY},
    Paint, Vertex,
};

#[derive(Clone, Copy)]
struct ScissorUniforms {
    mat: Transform2D<f32>,
    ext: Size2D<f32>,
    scale: Vector2D<f32>,
}

#[derive(Default, Clone, Copy)]
pub(crate) struct State {
    pub scissor_mat: Transform2D<f32>,
    pub scissor_ext: Size2D<f32>,
    pub scissor_uniforms: Option<ScissorUniforms>,
    pub xform: Option<Transform2D<f32>>,
    pub global_alpha: f32,
}

impl State {
    fn reset(&mut self, view_physical_size: Size2D<u32>) {
        self.scissor_mat = Transform2D::identity();
        self.scissor_ext = view_physical_size.cast();
        self.scissor_uniforms = None;
        self.xform = None;
        self.global_alpha = 1.0;
    }
}

pub struct Context {
    pub(crate) mesh_cache: MeshCache,
    pub(crate) mesh_cache_key: MeshCacheKey,
    pub(crate) tessellator: Tessellator,
    renderer: Renderer,
    state_stack: Vec1<State>,
    view_physical_size: Size2D<u32>,
    scale_factor: f32,
    pub(crate) dist_tol: f32,

    queued_items: Vec<QueuedDrawingCommand>,
    total_num_vertices: usize,
    needs_preparing: bool,
    pub(crate) antialiasing_enabled: bool,
}

impl Context {
    pub fn new(
        device: &wgpu::Device,
        texture_format: wgpu::TextureFormat,
        multisample: wgpu::MultisampleState,
        scale_factor: f32,
        antialiasing_enabled: bool,
    ) -> Self {
        assert!(scale_factor > 0.0);

        let mut tessellator = Tessellator::new();
        tessellator.set_scale_factor(scale_factor);
        let dist_tol = tessellator.dist_tol();

        Self {
            mesh_cache: MeshCache::new(),
            mesh_cache_key: MeshCacheKey::new(),
            tessellator,
            renderer: Renderer::new(device, texture_format, multisample),
            state_stack: Vec1::with_capacity(State::default(), INIT_STATE_CAPACITY),
            view_physical_size: Size2D::default(),
            scale_factor,
            dist_tol,

            queued_items: Vec::with_capacity(INIT_ITEMS_CAPACITY),
            total_num_vertices: 0,
            needs_preparing: false,
            antialiasing_enabled,
        }
    }

    pub fn begin_frame<'a>(
        &'a mut self,
        view_physical_size: Size2D<u32>,
        scale_factor: f32,
    ) -> ActiveContext<'a> {
        assert!(view_physical_size.width > 0);
        assert!(view_physical_size.height > 0);
        assert!(scale_factor > 0.0);

        let force_mesh_rebuild = if self.scale_factor != scale_factor {
            self.scale_factor = scale_factor;

            self.tessellator.set_scale_factor(scale_factor);

            true
        } else {
            false
        };

        self.mesh_cache.begin_frame(force_mesh_rebuild);
        self.queued_items.clear();
        self.total_num_vertices = 0;
        self.needs_preparing = true;
        self.view_physical_size = view_physical_size;

        self.state_stack.truncate(1).unwrap();
        self.state_stack.first_mut().reset(view_physical_size);

        ActiveContext { r: self }
    }

    fn queue_item(
        &mut self,
        mesh_id: MeshID,
        vert_range: Range<u32>,
        paint: Paint,
        offset: Vector2D<f32>,
        is_stroke: bool,
        mut stroke_width: f32,
        copy_verts: bool,
    ) -> QueuedDrawingCommand {
        let fringe_width = self.tessellator.fringe_width();
        let state = self.state_stack.last_mut();

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

        let alpha_mult = alpha_mult * state.global_alpha;

        let scissor_uniforms = if let Some(s) = &state.scissor_uniforms {
            s
        } else {
            state.scissor_uniforms = Some(
                if state.scissor_ext.width < -0.5 || state.scissor_ext.height < -0.5 {
                    ScissorUniforms {
                        mat: Transform2D::identity(),
                        ext: Size2D::new(1.0, 1.0),
                        scale: Vector2D::new(1.0, 1.0),
                    }
                } else {
                    ScissorUniforms {
                        mat: state.scissor_mat.inverse().unwrap(),
                        ext: state.scissor_ext,
                        scale: Vector2D::new(
                            (state.scissor_mat.m11 * state.scissor_mat.m11
                                + state.scissor_mat.m21 * state.scissor_mat.m21)
                                .sqrt()
                                * self.scale_factor,
                            (state.scissor_mat.m12 * state.scissor_mat.m12
                                + state.scissor_mat.m22 * state.scissor_mat.m22)
                                .sqrt()
                                * self.scale_factor,
                        ),
                    }
                },
            );

            state.scissor_uniforms.as_ref().unwrap()
        };

        let uniforms = match paint {
            Paint::SolidColor(mut color) => {
                color.a *= alpha_mult;

                // Premultiply color
                color.r *= color.a;
                color.g *= color.a;
                color.b *= color.a;

                ItemUniforms::new(
                    &scissor_uniforms.mat,
                    &None,
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
            Paint::Gradient(gradient) => {
                todo!()
            }
            Paint::Image(image) => {
                todo!()
            }
        };

        let command = QueuedDrawingCommand {
            mesh_id,
            is_stroke,
            vert_range,
            uniforms,
            copy_verts,
        };

        self.queued_items.push(command.clone());

        command
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

    #[inline]
    pub(crate) fn state(&self) -> &State {
        self.state_stack.last()
    }

    #[inline]
    pub(crate) fn state_mut(&mut self) -> &mut State {
        self.state_stack.last_mut()
    }
}

pub struct ActiveContext<'a> {
    r: &'a mut Context,
}

impl<'a> ActiveContext<'a> {
    /// Pushes and saves the current render state into the state stack.
    ///
    /// A matching [`ActiveContext::restore()`] must be used to restore the state.
    pub fn save(&mut self) {
        let last = self.r.state_stack.last().clone();
        self.r.state_stack.push(last);
    }

    /// Pops and restores current render state.
    pub fn restore(&mut self) {
        if self.r.state_stack.len() > 1 {
            self.r.state_stack.pop().unwrap();
        }
    }

    /// Resets current render state to default values. Does not affect the render
    /// state stack.
    pub fn reset(&mut self) {
        let view_physical_size = self.r.view_physical_size;
        self.r.state_mut().reset(view_physical_size);
    }

    /// Set the global alpha multiplier for the current render state.
    pub fn global_alpha(&mut self, alpha: f32) {
        self.r.state_mut().global_alpha = alpha.clamp(0.0, 1.0);
    }

    pub fn transform(&mut self, t: impl Into<Transform2D<f32>>) {
        let t: Transform2D<f32> = t.into();
        let state = self.r.state_mut();

        if let Some(xform) = &mut state.xform {
            *xform = t.then(xform);
        } else {
            state.xform = Some(t);
        }
    }

    pub fn reset_transform(&mut self) {
        self.r.state_mut().xform = None;
    }

    pub fn translate(&mut self, v: impl Into<Vector2D<f32>>) {
        let v: Vector2D<f32> = v.into();
        let state = self.r.state_mut();

        if let Some(xform) = &mut state.xform {
            *xform = xform.pre_translate(v);
        } else {
            state.xform = Some(Transform2D::translation(v.x, v.y));
        }
    }

    pub fn rotate(&mut self, angle: impl Into<Angle<f32>>) {
        let angle: Angle<f32> = angle.into();
        let state = self.r.state_mut();

        if let Some(xform) = &mut state.xform {
            *xform = xform.pre_rotate(angle);
        } else {
            state.xform = Some(Transform2D::rotation(angle));
        }
    }

    pub fn scale(&mut self, x: f32, y: f32) {
        let state = self.r.state_mut();

        if let Some(xform) = &mut state.xform {
            *xform = xform.pre_scale(x, y);
        } else {
            state.xform = Some(Transform2D::scale(x, y));
        }
    }

    pub fn skew_x(&mut self, angle: impl Into<Angle<f32>>) {
        let angle: Angle<f32> = angle.into();
        let state = self.r.state_mut();

        if let Some(xform) = &mut state.xform {
            *xform = crate::math::transform_skew_x(angle).then(&xform)
        } else {
            state.xform = Some(crate::math::transform_skew_x(angle));
        }
    }

    pub fn skew_y(&mut self, angle: impl Into<Angle<f32>>) {
        let angle: Angle<f32> = angle.into();
        let state = self.r.state_mut();

        if let Some(xform) = &mut state.xform {
            *xform = crate::math::transform_skew_y(angle).then(&xform)
        } else {
            state.xform = Some(crate::math::transform_skew_y(angle));
        }
    }

    pub fn current_transform(&self) -> Transform2D<f32> {
        self.r.state().xform.unwrap_or(Transform2D::default())
    }

    /// Begin constructing a new mesh
    pub fn begin_mesh<'b>(&'b mut self) -> MeshBuilder<'b> {
        MeshBuilder::new(&mut self.r)
    }

    /// Insert a raw mesh.
    pub fn raw_mesh(
        &mut self,
        fill_verts: Vec<Vertex>,
        stroke_verts: Vec<Vertex>,
        stroke_width: f32,
        anti_aliased: bool,
    ) -> RawMeshID {
        self.r
            .mesh_cache
            .insert_raw_mesh(fill_verts, stroke_verts, stroke_width, anti_aliased)
    }

    /// Insert/modify a raw mesh, re-using the allocations from the previous frame to improve
    /// performance.
    ///
    /// If `mesh_id == RawMeshID::default()` (a dangling ID), then a new mesh slot will be
    /// allocated and `mesh_id` will be replaced with the new valid ID.
    ///
    /// The inputs to the provided closure are `(fill_vertices, stroke_vertices)`, and the
    /// outputs are `(stroke_width, anti_aliased)`. Also note that `fill_vertices` and
    /// `stroke_vertices` are *NOT* automatically cleared.
    pub fn raw_mesh_reusing_alloc<F: FnOnce(&mut Vec<Vertex>, &mut Vec<Vertex>) -> (f32, bool)>(
        &mut self,
        mesh_id: &mut RawMeshID,
        f: F,
    ) {
        self.r.mesh_cache.raw_mesh_mut(mesh_id, f);
    }

    /// Returns `true` if a mesh with the given ID exists in the context, or `false`
    /// if the ID is invalid (i.e. it has been purged from the cache).
    ///
    /// If this returns `false`, then the mesh must be rebuilt.
    pub fn mesh_is_valid(&self, mesh_id: impl Into<MeshID>) -> bool {
        let mesh_id: MeshID = mesh_id.into();
        self.r.mesh_cache.contains(mesh_id)
    }

    pub fn paint_fill(
        &mut self,
        mesh_id: impl Into<MeshID>,
        paint: impl Into<Paint>,
    ) -> Option<QueuedDrawingCommand> {
        self.paint_fill_with_offset(mesh_id, paint, Vector2D::zero())
    }

    pub fn paint_stroke(
        &mut self,
        mesh_id: impl Into<MeshID>,
        paint: impl Into<Paint>,
    ) -> Option<QueuedDrawingCommand> {
        self.paint_stroke_with_offset(mesh_id, paint, Vector2D::zero())
    }

    pub fn paint_fill_with_offset(
        &mut self,
        mesh_id: impl Into<MeshID>,
        paint: impl Into<Paint>,
        offset: Vector2D<f32>,
    ) -> Option<QueuedDrawingCommand> {
        let mesh_id: MeshID = mesh_id.into();
        let paint: Paint = paint.into();

        let Some(mesh) = self.r.mesh_cache.get_mut(mesh_id) else {
            return None;
        };

        let (vert_range, copy_verts) = if let Some(vert_range) = mesh.fill_vert_range.clone() {
            (vert_range, false)
        } else {
            if mesh.fill_verts.is_empty() {
                return None;
            }

            let vert_range = self.r.total_num_vertices as u32
                ..(self.r.total_num_vertices + mesh.fill_verts.len()) as u32;
            mesh.fill_vert_range = Some(vert_range.clone());

            self.r.total_num_vertices += mesh.fill_verts.len();

            (vert_range, true)
        };

        Some(
            self.r
                .queue_item(mesh_id, vert_range, paint, offset, false, 0.0, copy_verts),
        )
    }

    pub fn paint_stroke_with_offset(
        &mut self,
        mesh_id: impl Into<MeshID>,
        paint: impl Into<Paint>,
        offset: Vector2D<f32>,
    ) -> Option<QueuedDrawingCommand> {
        let mesh_id: MeshID = mesh_id.into();
        let paint: Paint = paint.into();

        let Some(mesh) = self.r.mesh_cache.get_mut(mesh_id) else {
            return None;
        };

        let (vert_range, copy_verts) = if let Some(vert_range) = mesh.stroke_vert_range.clone() {
            (vert_range, false)
        } else {
            if mesh.stroke_verts.is_empty() {
                return None;
            }

            let vert_range = self.r.total_num_vertices as u32
                ..(self.r.total_num_vertices + mesh.stroke_verts.len()) as u32;
            mesh.stroke_vert_range = Some(vert_range.clone());

            self.r.total_num_vertices += mesh.stroke_verts.len();

            (vert_range, true)
        };

        let stroke_width = mesh.stroke_width;
        Some(self.r.queue_item(
            mesh_id,
            vert_range,
            paint,
            offset,
            true,
            stroke_width,
            copy_verts,
        ))
    }

    /// Cancel this [`ActiveContext`] and clear all queued items.
    pub fn cancel_frame(self) {
        self.r.queued_items.clear();
        self.r.total_num_vertices = 0;
        self.r.needs_preparing = false;
    }
}

/// An item that has been queued up to be sent to the GPU for drawing.
///
/// Instead of rebuilding and repainting an item that has not changed since the last
/// frame, you can simply re-add this struct using *TODO*.
///
/// Note this only works if the drawing command has been used last frame. To check
/// whether or not this drawing command is still valid (or if it needs rebuilding)
/// use *TODO*.
#[derive(Debug, Clone)]
pub struct QueuedDrawingCommand {
    pub(crate) mesh_id: MeshID,
    pub(crate) is_stroke: bool,
    pub(crate) vert_range: Range<u32>,
    pub(crate) uniforms: ItemUniforms,
    pub(crate) copy_verts: bool,
}

impl QueuedDrawingCommand {
    /// Offset the drawing commands by the given pixel amount.
    pub fn offset(&mut self, offset: impl Into<Vector2D<f32>>) {
        let offset: Vector2D<f32> = offset.into();

        self.uniforms.offset[0] += offset.x;
        self.uniforms.offset[1] += offset.y;
    }
}
