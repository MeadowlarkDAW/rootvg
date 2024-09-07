use std::f32::consts::{PI, TAU};
use std::num::NonZeroUsize;

use euclid::{
    default::{Point2D, Rect, Size2D, Transform2D, Vector2D},
    point2, vec2, Angle,
};
use vec1::Vec1;

use crate::{
    vert, Align, BlendFactor, Color, CompositeOperation, CompositeOperationState, LineCap, Paint,
    Scissor, Vertex, Winding,
};

//const INIT_FONTIMAGE_SIZE: usize = 512;
//const MAX_FONTIMAGE_SIZE: usize = 2048;
//const MAX_FONTIMAGES: usize = 4;

const INIT_COMMANDS_SIZE: usize = 64;
const INIT_VERTS_SIZE: usize = 256;
const INIT_STATES: usize = 64;

#[derive(Default, Debug, Clone, Copy, PartialEq)]
struct State {
    composite_operation: CompositeOperationState,
    shape_anti_alias: bool,
    fill: Paint,
    stroke: Paint,
    stroke_width: f32,
    miter_limit: f32,
    line_join: LineCap,
    line_cap: LineCap,
    alpha: f32,
    xform: Transform2D<f32>,
    scissor: Scissor,
    font_size: f32,
    letter_spacing: f32,
    line_height: f32,
    font_blur: f32,
    text_align: Align,
    font_id: i32,
}

impl State {
    fn reset(&mut self) {
        self.fill = Paint::new(crate::color::WHITE);
        self.stroke = Paint::new(crate::color::BLACK);

        self.composite_operation = CompositeOperation::SourceOver.state();
        self.shape_anti_alias = true;
        self.stroke_width = 1.0;
        self.miter_limit = 10.0;
        self.line_cap = LineCap::Butt;
        self.line_join = LineCap::Miter;
        self.alpha = 1.0;
        self.xform = Transform2D::identity();

        self.scissor = Scissor::new();

        self.font_size = 16.0;
        self.letter_spacing = 0.0;
        self.line_height = 1.0;
        self.font_blur = 0.0;
        self.text_align = Align::HALIGN_LEFT | Align::VALIGN_BASELINE;
        self.font_id = 0;
    }
}

pub struct Context {
    // NVGparams params;
    commands: Vec<Command>,
    command_pos: Point2D<f32>,
    states: Vec1<State>,
    cache: PathCache,
    tess_tol: f32,
    dist_tol: f32,
    fringe_width: f32,
    device_px_ratio: f32,
    draw_call_count: usize,
    fill_tri_count: usize,
    stroke_tri_count: usize,
    text_tri_count: usize,
}

impl Context {
    pub fn new() -> Self {
        let mut new_self = Self {
            commands: Vec::with_capacity(INIT_COMMANDS_SIZE),
            command_pos: Point2D::default(),
            states: Vec1::with_capacity(State::default(), INIT_STATES),
            cache: PathCache::new(),
            tess_tol: 0.0,
            dist_tol: 0.0,
            fringe_width: 0.0,
            device_px_ratio: 0.0,
            draw_call_count: 0,
            fill_tri_count: 0,
            stroke_tri_count: 0,
            text_tri_count: 0,
        };

        new_self.reset();

        new_self.set_device_pixel_ratio(1.0);

        // if (ctx->params.renderCreate(ctx->params.userPtr) == 0) goto error;

        // init font rendering

        // create font texture

        new_self
    }

    pub fn set_device_pixel_ratio(&mut self, ratio: f32) {
        self.tess_tol = 0.25 / ratio;
        self.dist_tol = 0.01 / ratio;
        self.fringe_width = 1.0 / ratio;
        self.device_px_ratio = ratio;
    }

    pub fn begin_frame(&mut self, window_width: f32, window_height: f32, device_pixel_ratio: f32) {
        self.states.truncate_nonzero(NonZeroUsize::new(1).unwrap());
        self.reset();

        self.set_device_pixel_ratio(device_pixel_ratio);

        // ctx->params.renderViewport(ctx->params.userPtr, windowWidth, windowHeight, devicePixelRatio);

        self.draw_call_count = 0;
        self.fill_tri_count = 0;
        self.stroke_tri_count = 0;
        self.text_tri_count = 0;
    }

    pub fn cancel_frame(&mut self) {
        // ctx->params.renderCancel(ctx->params.userPtr);
    }

    pub fn end_frame(&mut self) {
        // ctx->params.renderFlush(ctx->params.userPtr);

        // font stuff
    }

    pub fn save(&mut self) {
        let last_state = *self.states.last();
        self.states.push(last_state);
    }

    pub fn restore(&mut self) {
        if self.states.len() > 1 {
            self.states.pop().unwrap();
        }
    }

    pub fn reset(&mut self) {
        self.state_mut().reset();
    }

    pub fn anti_alias(&mut self, enabled: bool) {
        self.state_mut().shape_anti_alias = enabled;
    }

    pub fn stroke_width(&mut self, width: f32) {
        self.state_mut().stroke_width = width;
    }

    pub fn miter_limit(&mut self, limit: f32) {
        self.state_mut().miter_limit = limit;
    }

    pub fn line_cap(&mut self, cap: LineCap) {
        self.state_mut().line_cap = cap;
    }

    pub fn line_join(&mut self, join: LineCap) {
        self.state_mut().line_join = join;
    }

    pub fn global_alpha(&mut self, alpha: f32) {
        self.state_mut().alpha = alpha;
    }

    pub fn transform(&mut self, t: &Transform2D<f32>) {
        let state = self.state_mut();
        state.xform = t.then(&state.xform);
    }

    pub fn reset_transform(&mut self) {
        self.state_mut().xform = Transform2D::identity();
    }

    pub fn translate(&mut self, v: impl Into<Vector2D<f32>>) {
        let state = self.state_mut();
        state.xform = state.xform.pre_translate(v.into());
    }

    pub fn rotate(&mut self, angle: impl Into<Angle<f32>>) {
        let state = self.state_mut();
        state.xform = state.xform.pre_rotate(angle.into());
    }

    pub fn skew_x(&mut self, angle: impl Into<Angle<f32>>) {
        let state = self.state_mut();
        state.xform = crate::math::transform_skew_x(angle.into()).then(&state.xform);
    }

    pub fn skew_y(&mut self, angle: impl Into<Angle<f32>>) {
        let state = self.state_mut();
        state.xform = crate::math::transform_skew_y(angle.into()).then(&state.xform);
    }

    pub fn scale(&mut self, x: f32, y: f32) {
        let state = self.state_mut();
        state.xform = state.xform.pre_scale(x, y);
    }

    pub fn current_transform(&self) -> &Transform2D<f32> {
        &self.state().xform
    }

    pub fn stroke_color(&mut self, color: impl Into<Color>) {
        self.state_mut().stroke = Paint::new(color);
    }

    pub fn stroke_paint(&mut self, paint: Paint) {
        let state = self.state_mut();
        state.stroke = paint;
        state.stroke.xform = state.stroke.xform.then(&state.xform);
    }

    pub fn fill_color(&mut self, color: impl Into<Color>) {
        self.state_mut().fill = Paint::new(color);
    }

    pub fn fill_paint(&mut self, paint: Paint) {
        let state = self.state_mut();
        state.fill = paint;
        state.fill.xform = state.fill.xform.then(&state.xform);
    }

    pub fn scissor(&mut self, rect: impl Into<Rect<f32>>) {
        let state = self.state_mut();

        let mut rect: Rect<f32> = rect.into();

        rect.size.width = rect.size.width.max(0.0);
        rect.size.height = rect.size.height.max(0.0);

        state.scissor.xform = Transform2D::identity();
        state.scissor.xform.m31 = rect.min_x() + rect.width() * 0.5;
        state.scissor.xform.m32 = rect.min_y() + rect.height() * 0.5;
        state.scissor.xform = state.scissor.xform.then(&state.xform);

        state.scissor.extent = Size2D::new(rect.width() * 0.5, rect.height() * 0.5)
    }

    // TODO: Custom error
    pub fn intersect_scissor(&mut self, rect: impl Into<Rect<f32>>) -> Result<(), ()> {
        let state = self.state_mut();

        let rect: Rect<f32> = rect.into();

        // If no previous scissor has been set, set the scissor as current scissor.
        if state.scissor.extent.width < 0.0 {
            self.scissor(rect);
            return Ok(());
        }

        // Transform the current scissor rect into current transform space.
        // If there is difference in rotation, this will be approximation.
        let mut p_xform = state.scissor.xform;
        let ex = state.scissor.extent.width;
        let ey = state.scissor.extent.height;
        let Some(inv_xform) = state.xform.inverse() else {
            return Err(());
        };
        p_xform = p_xform.then(&inv_xform);
        let tex = ex * p_xform.m11.abs() + ey * p_xform.m21.abs();
        let tey = ex * p_xform.m12.abs() + ey * p_xform.m22.abs();

        let Some(intersection_rect) = rect.intersection(&Rect::new(
            point2(p_xform.m31 - tex, p_xform.m32 - tey),
            Size2D::new(tex * 2.0, tey * 2.0),
        )) else {
            return Err(());
        };

        self.scissor(intersection_rect);

        Ok(())
    }

    pub fn reset_scissor(&mut self) {
        self.state_mut().scissor = Scissor::new();
    }

    pub fn global_composite_operation(&mut self, op: CompositeOperation) {
        self.state_mut().composite_operation = op.state();
    }

    pub fn global_composite_blend_func(
        &mut self,
        src_factor: BlendFactor,
        dst_factor: BlendFactor,
    ) {
        self.global_composite_blend_func_separate(src_factor, src_factor, dst_factor, dst_factor);
    }

    pub fn global_composite_blend_func_separate(
        &mut self,
        src_rgb: BlendFactor,
        dst_rgb: BlendFactor,
        src_alpha: BlendFactor,
        dst_alpha: BlendFactor,
    ) {
        self.state_mut().composite_operation = CompositeOperationState {
            src_rgb,
            dst_rgb,
            src_alpha,
            dst_alpha,
        };
    }

    pub fn begin_path(&mut self) {
        self.commands.clear();
        self.clear_path_cache();
    }

    pub fn close_path(&mut self) {
        self.commands.push(Command::Close);
    }

    pub fn path_winding(&mut self, dir: Winding) {
        self.commands.push(Command::Winding(dir));
    }

    pub fn fill(&mut self) {
        let mut fill_paint = self.state().fill;

        self.flatten_paths();

        if self.state().shape_anti_alias {
            self.expand_fill(self.fringe_width, LineCap::Miter, 2.4);
        } else {
            self.expand_fill(0.0, LineCap::Miter, 2.4);
        }

        let state = self.state();

        // Apply global alpha
        fill_paint.inner_color.a *= state.alpha;
        fill_paint.outer_color.a *= state.alpha;

        // render fill

        // Count triangles
        for path in self.cache.paths.iter() {
            self.fill_tri_count += path.num_fill_verts - 2;
            self.fill_tri_count += path.num_stroke_verts - 2;
            self.draw_call_count += 2;
        }
    }

    pub fn stroke(&mut self) {
        let state = self.state();

        let scale = transform_average_scale(&state.xform);
        let mut stroke_width = (state.stroke_width * scale).clamp(0.0, 200.0);
        let mut stroke_paint = state.stroke;

        if stroke_width < self.fringe_width {
            // If the stroke width is less than pixel size, use alpha to emulate coverage.
            // Since coverage is area, scale by alpha*alpha.
            let alpha = (stroke_width / self.fringe_width).clamp(0.0, 1.0);
            stroke_paint.inner_color.a *= alpha * alpha;
            stroke_paint.outer_color.a *= alpha * alpha;
            stroke_width = self.fringe_width;
        }

        // Apply global alpha
        stroke_paint.inner_color.a *= state.alpha;
        stroke_paint.outer_color.a *= state.alpha;

        self.flatten_paths();

        let state = self.state();

        if state.shape_anti_alias {
            self.expand_stroke(
                stroke_width * 0.5,
                self.fringe_width,
                state.line_cap,
                state.line_join,
                state.miter_limit,
            );
        } else {
            self.expand_stroke(
                stroke_width * 0.5,
                0.0,
                state.line_cap,
                state.line_join,
                state.miter_limit,
            );
        }

        // render stroke

        // Count triangles
        for path in self.cache.paths.iter() {
            self.stroke_tri_count += path.num_stroke_verts - 2;
            self.draw_call_count += 1;
        }
    }

    fn clear_path_cache(&mut self) {
        self.cache.clear();
    }

    fn flatten_paths(&mut self) {}

    #[inline]
    fn state(&self) -> &State {
        self.states.last()
    }

    #[inline]
    fn state_mut(&mut self) -> &mut State {
        self.states.last_mut()
    }
}
