use std::f32::consts::{PI, TAU};
use std::num::NonZeroUsize;

use euclid::{
    default::{Point2D, Rect, Size2D, Transform2D, Vector2D},
    point2, vec2, Angle,
};
use vec1::Vec1;

use crate::{
    vert, Align, BlendFactor, Color, CompositeOperation, CompositeOperationState, LineCap, Paint,
    Path, Scissor, Vertex, Winding,
};

//const INIT_FONTIMAGE_SIZE: usize = 512;
//const MAX_FONTIMAGE_SIZE: usize = 2048;
//const MAX_FONTIMAGES: usize = 4;

const INIT_COMMANDS_SIZE: usize = 64;
const INIT_POINTS_SIZE: usize = 128;
const INIT_PATHS_SIZE: usize = 16;
const INIT_VERTS_SIZE: usize = 256;
const INIT_STATES: usize = 64;

/// Length proportional to radius of a cubic bezier handle for 90deg arcs
const KAPPA90: f32 = 0.5522847493;
const ONE_MINUS_KAPPA90: f32 = 1.0 - KAPPA90;

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct PointFlags: u8 {
        const CORNER = 1 << 0;
        const LEFT = 1 << 1;
        const BEVEL = 1 << 2;
        const INNER_BEVEL = 1 << 3;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Command {
    MoveTo(Point2D<f32>),
    LineTo(Point2D<f32>),
    BezierTo {
        pos: Point2D<f32>,
        h1_pos: Point2D<f32>,
        h2_pos: Point2D<f32>,
    },
    Close,
    Winding(Winding),
}

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

#[derive(Debug, Clone, Copy, PartialEq)]
struct PathPoint {
    pos: Point2D<f32>,
    delta: Vector2D<f32>,
    len: f32,
    dm: Vector2D<f32>,
    flags: PointFlags,
}

impl PathPoint {
    fn new(pos: Point2D<f32>, flags: PointFlags) -> Self {
        Self {
            pos,
            delta: Vector2D::zero(),
            len: 0.0,
            dm: Vector2D::zero(),
            flags,
        }
    }
}

struct PathCache {
    points: Vec<PathPoint>,
    paths: Vec<Path>,
    verts: Vec<Vertex>,
    bounds_tl: Point2D<f32>,
    bounds_br: Point2D<f32>,
}

impl PathCache {
    fn new() -> Self {
        Self {
            points: Vec::with_capacity(INIT_POINTS_SIZE),
            paths: Vec::with_capacity(INIT_PATHS_SIZE),
            verts: Vec::with_capacity(INIT_VERTS_SIZE),
            bounds_tl: Point2D::default(),
            bounds_br: Point2D::default(),
        }
    }

    fn clear(&mut self) {
        self.points.clear();
        self.paths.clear();
        self.verts.clear();
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

    pub fn move_to(&mut self, pos: impl Into<Point2D<f32>>) {
        self.command_pos = pos.into();
        let pos = self.state().xform.transform_point(self.command_pos);
        self.commands.push(Command::MoveTo(pos));
    }

    pub fn line_to(&mut self, pos: impl Into<Point2D<f32>>) {
        self.command_pos = pos.into();
        let pos = self.state().xform.transform_point(self.command_pos);
        self.commands.push(Command::LineTo(pos));
    }

    pub fn bezier_to(
        &mut self,
        pos: impl Into<Point2D<f32>>,
        h1_pos: impl Into<Point2D<f32>>,
        h2_pos: impl Into<Point2D<f32>>,
    ) {
        self.command_pos = pos.into();

        let state = self.state();

        let pos = state.xform.transform_point(self.command_pos);
        let h1_pos = state.xform.transform_point(h1_pos.into());
        let h2_pos = state.xform.transform_point(h2_pos.into());

        self.commands.push(Command::BezierTo {
            pos,
            h1_pos,
            h2_pos,
        });
    }

    pub fn quad_to(&mut self, pos: impl Into<Point2D<f32>>, c: impl Into<Point2D<f32>>) {
        let pos: Point2D<f32> = pos.into();
        let c: Point2D<f32> = c.into();

        self.bezier_to(
            pos,
            self.command_pos + ((c - self.command_pos) * (2.0 / 3.0)),
            pos + ((c - pos) * (2.0 / 3.0)),
        );
    }

    pub fn arc_to(
        &mut self,
        p1: impl Into<Point2D<f32>>,
        p2: impl Into<Point2D<f32>>,
        radius: f32,
    ) {
        if self.commands.is_empty() {
            return;
        }

        let p1: Point2D<f32> = p1.into();
        let p2: Point2D<f32> = p2.into();

        let p0 = self.command_pos;

        // Handle degenerate cases
        if point_approx_equals(p0, p1, self.dist_tol)
            || point_approx_equals(p1, p2, self.dist_tol)
            || dist_point_seg(p1, p0, p2) < self.dist_tol * self.dist_tol
            || radius < self.dist_tol
        {
            self.line_to(p1);
            return;
        }

        // Calculate tangential circle to lines (x0,y0)-(x1,y1) and (x1,y1)-(x2,y2)
        let mut d0 = p0 - p1;
        let mut d1 = p2 - p1;
        normalize(&mut d0);
        normalize(&mut d1);
        let a = (d0.x * d1.x + d0.y * d1.y).cos();
        let d = radius / (a * 0.5).tan();

        if d > 10_000.0 {
            self.line_to(p1);
            return;
        }

        let (center, a0, a1, dir) = if d0.cross(d1) > 0.0 {
            let center = point2(
                p1.x + d0.x * d + d0.y * radius,
                p1.y + d0.y * d - d0.x * radius,
            );
            let a0 = d0.x.atan2(-d0.y);
            let a1 = (-d1.x).atan2(d1.y);
            (center, a0, a1, Winding::CW)
        } else {
            let center = point2(
                p1.x + d0.x * d - d0.y * radius,
                p1.y + d0.y * d + d0.x * radius,
            );
            let a0 = (-d0.x).atan2(d0.y);
            let a1 = d1.x.atan2(-d1.y);
            (center, a0, a1, Winding::CCW)
        };

        self.arc(center, Angle::radians(a0), Angle::radians(a1), radius, dir);
    }

    pub fn close_path(&mut self) {
        self.commands.push(Command::Close);
    }

    pub fn path_winding(&mut self, dir: Winding) {
        self.commands.push(Command::Winding(dir));
    }

    pub fn arc(
        &mut self,
        center: impl Into<Point2D<f32>>,
        angle_0: impl Into<Angle<f32>>,
        angle_1: impl Into<Angle<f32>>,
        radius: f32,
        dir: Winding,
    ) {
        self.barc(center, angle_0, angle_1, radius, dir, LineCap::Round)
    }

    pub fn barc(
        &mut self,
        center: impl Into<Point2D<f32>>,
        angle_0: impl Into<Angle<f32>>,
        angle_1: impl Into<Angle<f32>>,
        radius: f32,
        dir: Winding,
        join: LineCap,
    ) {
        let center: Point2D<f32> = center.into();
        let angle_0: Angle<f32> = angle_0.into();
        let angle_1: Angle<f32> = angle_1.into();

        // Clamp angles
        let mut da = angle_1 - angle_0;
        if dir == Winding::CW {
            if da.radians.abs() >= TAU {
                da.radians = TAU;
            } else {
                while da.radians < 0.0 {
                    da.radians += TAU;
                }
            }
        } else {
            if da.radians.abs() >= TAU {
                da.radians = -TAU;
            } else {
                while da.radians > 0.0 {
                    da.radians -= TAU;
                }
            }
        }

        // Split arc into max 90 degree segments
        let num_divs =
            1.max(5.min((da.radians.abs() * (1.0 / (PI * 0.5)) + 0.5) as isize)) as usize;
        let hda = (da / num_divs as f32) * 0.5;

        let mut kappa = (4.0 / 3.0 * (1.0 - hda.radians.cos()) / hda.radians.sin()).abs();
        if dir == Winding::CCW {
            kappa = -kappa;
        }

        let mut prev_pos = Point2D::<f32>::zero();
        let mut prev_tanv = Vector2D::<f32>::zero();

        let num_divs_recip = (num_divs as f32).recip();

        for i in 0..num_divs {
            let a = angle_0.radians + da.radians * (i as f32 * num_divs_recip);
            let delta = vec2(a.cos(), a.sin());
            let pos = center + (delta * radius);
            let tanv = vec2(-delta.y * radius * kappa, delta.x * radius * kappa);

            if i == 0 {
                if join != LineCap::Butt && !self.commands.is_empty() {
                    self.line_to(pos);
                } else {
                    self.move_to(pos);
                }
            } else {
                self.bezier_to(pos, prev_pos + prev_tanv, pos - tanv);
            }

            prev_pos = pos;
            prev_tanv = tanv;
        }
    }

    pub fn rect(&mut self, rect: impl Into<Rect<f32>>) {
        let rect: Rect<f32> = rect.into();

        self.move_to(rect.origin);
        self.line_to(point2(rect.min_x(), rect.max_y()));
        self.line_to(rect.max());
        self.line_to(point2(rect.max_x(), rect.min_y()));
        self.close_path();
    }

    pub fn rounded_rect(&mut self, rect: impl Into<Rect<f32>>, radius: f32) {
        self.rounded_rect_varying(rect, radius, radius, radius, radius);
    }

    pub fn rounded_rect_varying(
        &mut self,
        rect: impl Into<Rect<f32>>,
        rad_top_left: f32,
        rad_top_right: f32,
        rad_bottom_right: f32,
        rad_bottom_left: f32,
    ) {
        let rect: Rect<f32> = rect.into();

        if rad_top_left < 0.1
            && rad_top_right < 0.1
            && rad_bottom_right < 0.1
            && rad_bottom_left < 0.1
        {
            self.rect(rect);
        } else {
            let half_size = rect.size * 0.5;

            let sign_of_width = sign_of(rect.width());
            let sign_of_height = sign_of(rect.height());

            let bl: Vector2D<f32> = vec2(
                rad_bottom_left.min(half_size.width) * sign_of_width,
                rad_bottom_left.min(half_size.height) * sign_of_height,
            );
            let br: Vector2D<f32> = vec2(
                rad_bottom_right.min(half_size.width) * sign_of_width,
                rad_bottom_right.min(half_size.height) * sign_of_height,
            );
            let tl: Vector2D<f32> = vec2(
                rad_top_left.min(half_size.width) * sign_of_width,
                rad_top_left.min(half_size.height) * sign_of_height,
            );
            let tr: Vector2D<f32> = vec2(
                rad_top_right.min(half_size.width) * sign_of_width,
                rad_top_right.min(half_size.height) * sign_of_height,
            );

            self.move_to(point2(rect.min_x(), rect.min_y() + tl.y));

            self.line_to(point2(rect.min_x(), rect.max_y() - bl.y));
            self.bezier_to(
                point2(rect.min_x() + bl.x, rect.max_y()),
                point2(rect.min_x(), rect.max_y() - bl.y * ONE_MINUS_KAPPA90),
                point2(rect.min_x() + bl.x * ONE_MINUS_KAPPA90, rect.max_y()),
            );

            self.line_to(point2(rect.max_x() - br.x, rect.max_y()));
            self.bezier_to(
                point2(rect.max_x(), rect.max_y() - br.y),
                point2(rect.max_x() - br.x * ONE_MINUS_KAPPA90, rect.max_y()),
                point2(rect.max_x(), rect.max_y() - br.y * ONE_MINUS_KAPPA90),
            );

            self.line_to(point2(rect.max_x(), rect.min_y() + tr.y));
            self.bezier_to(
                point2(rect.max_x() - tr.x, rect.min_y()),
                point2(rect.max_x(), rect.min_y() + tr.y * ONE_MINUS_KAPPA90),
                point2(rect.max_x() - tr.x * ONE_MINUS_KAPPA90, rect.min_y()),
            );

            self.line_to(point2(rect.min_x() + tl.x, rect.min_y()));
            self.bezier_to(
                point2(rect.min_x(), rect.min_y() + tl.y),
                point2(rect.min_x() + tl.x * ONE_MINUS_KAPPA90, rect.min_y()),
                point2(rect.min_x(), rect.min_y() + tl.y * ONE_MINUS_KAPPA90),
            );

            self.close_path();
        }
    }

    pub fn circle(&mut self, center: impl Into<Point2D<f32>>, radius: f32) {
        self.ellipse(center, radius, radius);
    }

    pub fn ellipse(&mut self, center: impl Into<Point2D<f32>>, rx: f32, ry: f32) {
        let center: Point2D<f32> = center.into();

        self.move_to(point2(center.x - rx, center.y));
        self.bezier_to(
            point2(center.x, center.y + ry),
            point2(center.x - rx, center.y + ry * KAPPA90),
            point2(center.x - rx * KAPPA90, center.y + ry),
        );
        self.bezier_to(
            point2(center.x + rx, center.y),
            point2(center.x + rx * KAPPA90, center.y + ry),
            point2(center.x + rx, center.y + ry * KAPPA90),
        );
        self.bezier_to(
            point2(center.x, center.y - ry),
            point2(center.x + rx, center.y - ry * KAPPA90),
            point2(center.x + rx * KAPPA90, center.y - ry),
        );
        self.bezier_to(
            point2(center.x - rx, center.y),
            point2(center.x - rx * KAPPA90, center.y - ry),
            point2(center.x - rx, center.y - ry * KAPPA90),
        );
        self.close_path();
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

    fn add_point(&mut self, pos: Point2D<f32>, flags: PointFlags) {
        let Some(path) = self.cache.paths.last_mut() else {
            return;
        };

        if path.num_points > 0 {
            if let Some(prev_p) = self.cache.points.last_mut() {
                if point_approx_equals(prev_p.pos, pos, self.dist_tol) {
                    prev_p.flags |= flags;
                    return;
                }
            }
        }

        self.cache.points.push(PathPoint::new(pos, flags));
        path.num_points += 1;
    }

    fn _close_path(&mut self) {
        let Some(path) = self.cache.paths.last_mut() else {
            return;
        };

        path.closed = true;
    }

    fn _path_winding(&mut self, winding: Winding) {
        let Some(path) = self.cache.paths.last_mut() else {
            return;
        };

        path.winding = winding;
    }

    fn tesselate_bezier(
        &mut self,
        p1: Point2D<f32>,
        p2: Point2D<f32>,
        p3: Point2D<f32>,
        p4: Point2D<f32>,
        level: usize,
        flags: PointFlags,
    ) {
        if level > 10 {
            return;
        }

        let p12 = (p1 + p2.to_vector()) * 0.5;
        let p23 = (p2 + p3.to_vector()) * 0.5;
        let p34 = (p3 + p4.to_vector()) * 0.5;
        let p123 = (p12 + p23.to_vector()) * 0.5;

        let dp = p4 - p4.to_vector();
        let d2 = ((p2.x - p4.x) * dp.y - (p2.y - p4.y) * dp.x).abs();
        let d3 = ((p3.x - p4.x) * dp.y - (p3.y - p4.y) * dp.x).abs();

        if (d2 + d3) * (d2 + d3) < self.tess_tol * (dp.x * dp.x + dp.y * dp.y) {
            self.add_point(p4, flags);
            return;
        }

        let p234 = (p23 + p34.to_vector()) * 0.5;
        let p1234 = (p123 + p234.to_vector()) * 0.5;

        self.tesselate_bezier(p1, p12, p123, p1234, level + 1, PointFlags::empty());
        self.tesselate_bezier(p1234, p234, p34, p4, level + 1, flags)
    }

    fn flatten_paths(&mut self) {
        if !self.cache.paths.is_empty() {
            return;
        }

        let mut commands = Vec::new();
        std::mem::swap(&mut commands, &mut self.commands);

        // flatten
        for cmd in commands.iter().copied() {
            match cmd {
                Command::MoveTo(pos) => {
                    self.cache.paths.push(Path::new(self.cache.points.len()));
                    self.add_point(pos, PointFlags::CORNER);
                }
                Command::LineTo(pos) => {
                    self.add_point(pos, PointFlags::CORNER);
                }
                Command::BezierTo {
                    pos,
                    h1_pos,
                    h2_pos,
                } => {
                    if let Some(last_point) = self.cache.points.last() {
                        self.tesselate_bezier(
                            last_point.pos,
                            pos,
                            h1_pos,
                            h2_pos,
                            0,
                            PointFlags::CORNER,
                        );
                    }
                }
                Command::Close => {
                    self._close_path();
                }
                Command::Winding(winding) => {
                    self._path_winding(winding);
                }
            }
        }

        std::mem::swap(&mut commands, &mut self.commands);

        self.cache.bounds_tl = point2(1e6, 1e6);
        self.cache.bounds_br = point2(-1e6, -1e6);

        // Calculate the direction and length of line segments.
        for path in self.cache.paths.iter_mut() {
            if path.num_points == 0 {
                continue;
            }

            let mut pts = &mut self.cache.points
                [path.point_start_index..path.point_start_index + path.num_points];

            // If the first and last points are the same, remove the last, mark as closed path.
            if pts.len() > 1 {
                if point_approx_equals(
                    pts.first().unwrap().pos,
                    pts.last().unwrap().pos,
                    self.dist_tol,
                ) {
                    path.closed = true;
                    let pts_len = pts.len();
                    pts = &mut pts[0..pts_len - 1];
                }
            } else {
                path.closed = false;
            }

            // Enforce winding
            if pts.len() > 2 {
                let area = poly_area(pts);
                if (path.winding == Winding::CCW && area < 0.0)
                    || (path.winding == Winding::CW && area > 0.0)
                {
                    pts.reverse();
                }
            }

            let mut process_pt = |p: &mut PathPoint, next_pos: Point2D<f32>| {
                // Calculate segment direction and length
                p.delta = next_pos - p.pos;
                p.len = normalize(&mut p.delta);

                // Update bounds
                self.cache.bounds_tl = self.cache.bounds_tl.min(p.pos);
                self.cache.bounds_br = self.cache.bounds_br.max(p.pos);
            };

            let first_pos = pts.first().unwrap().pos;
            process_pt(pts.last_mut().unwrap(), first_pos);

            for i in 1..pts.len() {
                let next_pos = pts[i].pos;
                process_pt(&mut pts[i - 1], next_pos);
            }
        }
    }

    fn calculate_joins(&mut self, w: f32, line_join: LineCap, miter_limit: f32) {
        let iw = if w > 0.0 { 1.0 / w } else { 0.0 };

        // Calculate which joins needs extra vertices to append, and gather vertex count.
        for path in self.cache.paths.iter_mut() {
            if path.num_points == 0 {
                continue;
            }

            let pts = &mut self.cache.points
                [path.point_start_index..path.point_start_index + path.num_points];

            let mut num_left_turns: usize = 0;

            let mut process_pt = |p: &mut PathPoint, prev_delta: Vector2D<f32>, prev_len: f32| {
                let dl0 = vec2(prev_delta.y, -prev_delta.x);
                let dl1 = vec2(p.delta.y, -p.delta.x);

                // Calculate extrusions
                p.dm = (dl0 + dl1) * 0.5;
                let dmr2 = p.dm.x * p.dm.x + p.dm.y + p.dm.y;
                if dmr2 > 0.000001 {
                    let scale = (1.0 / dmr2).min(600.0);
                    p.dm *= scale;
                }

                // Clear flags, but keep the corner
                p.flags = p.flags.intersection(PointFlags::CORNER);

                // Keep track of left turns
                let cross = p.delta.x * prev_delta.y - prev_delta.x * p.delta.y;
                if cross > 0.0 {
                    num_left_turns += 1;
                    p.flags.insert(PointFlags::LEFT);
                }

                // Calculate if we should use bevel or miter for inner join
                let limit = 1.0_f32.max(prev_len.min(p.len) * iw);
                if (dmr2 * limit * limit) < 1.0 {
                    p.flags.insert(PointFlags::INNER_BEVEL);
                }

                // Check to see if the corner needs to be beveled
                if p.flags.contains(PointFlags::CORNER) {
                    if (dmr2 * miter_limit * miter_limit) < 1.0
                        || line_join == LineCap::Bevel
                        || line_join == LineCap::Round
                    {
                        p.flags.insert(PointFlags::BEVEL);
                    }
                }

                if p.flags
                    .intersects(PointFlags::BEVEL | PointFlags::INNER_BEVEL)
                {
                    path.num_bevels += 1;
                }
            };

            let last_delta = pts.last().unwrap().delta;
            let last_len = pts.last().unwrap().len;
            process_pt(pts.first_mut().unwrap(), last_delta, last_len);

            for i in 1..pts.len() {
                let prev_delta = pts[i - 1].delta;
                let prev_len = pts[i - 1].len;
                process_pt(&mut pts[i], prev_delta, prev_len);
            }

            path.convex = num_left_turns == path.num_points;
        }
    }

    fn expand_stroke(
        &mut self,
        w: f32,
        fringe: f32,
        line_cap: LineCap,
        line_join: LineCap,
        miter_limit: f32,
    ) {
        let aa = fringe;
        let w = w + (aa * 0.5);

        // Calculate divisions per half circle
        let ncap = curve_divs(w, PI, self.tess_tol);

        // Disable the gradient used for antialiasing when antialiasing is not used
        let (u0, u1) = if aa == 0.0 { (0.5, 0.5) } else { (0.0, 1.0) };

        self.calculate_joins(w, line_join, miter_limit);

        // Calculate max vertex usage
        let mut vert_capacity = 0;
        for path in self.cache.paths.iter() {
            if line_join == LineCap::Round {
                // plus one for loop
                vert_capacity += (path.num_points + path.num_bevels * (ncap + 2) + 1) * 2;
            } else {
                // plus one for loop
                vert_capacity += (path.num_points + path.num_bevels * 5 + 1) * 2;
            }

            if path.closed {
                // space for caps
                if line_cap == LineCap::Round {
                    vert_capacity += (ncap * 2 + 2) * 2;
                } else {
                    vert_capacity += (3 + 3) * 2;
                }
            }
        }

        self.cache.verts.reserve(vert_capacity);

        for path in self.cache.paths.iter_mut() {
            if path.num_points < 2 {
                continue;
            }

            let pts = &self.cache.points
                [path.point_start_index..path.point_start_index + path.num_points];

            // Calculate fringe or stroke

            path.num_fill_verts = 0;
            path.stroke_vert_start_index = self.cache.verts.len();

            let join_pt = |p0: &PathPoint, p1: &PathPoint, verts: &mut Vec<Vertex>| {
                if p1
                    .flags
                    .intersects(PointFlags::BEVEL | PointFlags::INNER_BEVEL)
                {
                    if line_join == LineCap::Round {
                        round_join(verts, p0, p1, w, w, u0, u1, ncap);
                    } else {
                        bevel_join(verts, p0, p1, w, w, u0, u1);
                    }
                } else {
                    verts.push(vert(p1.pos + (p1.dm * w), point2(u0, 1.0)));
                    verts.push(vert(p1.pos - (p1.dm * w), point2(u1, 1.0)));
                }
            };

            if path.closed {
                // Join points
                join_pt(
                    pts.last().unwrap(),
                    pts.first().unwrap(),
                    &mut self.cache.verts,
                );
                for i in 1..pts.len() {
                    join_pt(&pts[i - 1], &pts[i], &mut self.cache.verts);
                }

                // Loop it
                let first_vert_pos = self.cache.verts[path.stroke_vert_start_index].pos;
                let second_vert_pos = self.cache.verts[path.stroke_vert_start_index + 1].pos;
                self.cache.verts.push(vert(first_vert_pos, point2(u0, 1.0)));
                self.cache
                    .verts
                    .push(vert(second_vert_pos, point2(u1, 1.0)));
            } else {
                // Add start cap
                let p0 = &pts[0];
                let p1 = &pts[1];
                let mut delta = p1.pos - p0.pos;
                normalize(&mut delta);
                match line_cap {
                    LineCap::Butt => {
                        butt_cap_start(&mut self.cache.verts, p0, delta, w, -aa * 0.5, aa, u0, u1)
                    }
                    LineCap::Square => {
                        butt_cap_start(&mut self.cache.verts, p0, delta, w, w - aa, aa, u0, u1)
                    }
                    LineCap::Round => {
                        round_cap_start(&mut self.cache.verts, p0, delta, w, ncap, u0, u1)
                    }
                    _ => {}
                }

                // Join points
                for i in 1..(pts.len() - 1) {
                    join_pt(&pts[i - 1], &pts[i], &mut self.cache.verts);
                }

                // Add end cap
                let p0 = &pts[pts.len() - 2];
                let p1 = &pts[pts.len() - 1];
                let mut delta = p1.pos - p0.pos;
                normalize(&mut delta);
                match line_cap {
                    LineCap::Butt => {
                        butt_cap_end(&mut self.cache.verts, p0, delta, w, -aa * 0.5, aa, u0, u1)
                    }
                    LineCap::Square => {
                        butt_cap_end(&mut self.cache.verts, p0, delta, w, w - aa, aa, u0, u1)
                    }
                    LineCap::Round => {
                        round_cap_end(&mut self.cache.verts, p0, delta, w, ncap, u0, u1)
                    }
                    _ => {}
                }
            };

            path.num_stroke_verts = self.cache.verts.len() - path.stroke_vert_start_index;
        }
    }

    fn expand_fill(&mut self, w: f32, line_join: LineCap, miter_limit: f32) {
        let aa = self.fringe_width;
        let fringe = w > 0.0;

        self.calculate_joins(w, line_join, miter_limit);

        // Calculate max vertex usage
        let mut vert_capacity = 0;
        for path in self.cache.paths.iter() {
            vert_capacity += path.num_points + path.num_bevels + 1;

            if fringe {
                // plus one for loop
                vert_capacity += (path.num_points + path.num_bevels * 5 + 1) * 2;
            }
        }

        self.cache.verts.reserve(vert_capacity);

        let convex = self.cache.paths.first().map(|p| p.convex).unwrap_or(false);

        for path in self.cache.paths.iter_mut() {
            if path.num_points < 2 {
                continue;
            }

            let pts = &self.cache.points
                [path.point_start_index..path.point_start_index + path.num_points];

            // Calculate shape vertices
            let woff = 0.5 * aa;
            path.fill_vert_start_index = self.cache.verts.len();

            if fringe {
                // Looping
                let process_pair = |p0: &PathPoint, p1: &PathPoint, verts: &mut Vec<Vertex>| {
                    if p1.flags.contains(PointFlags::BEVEL) {
                        let dl0 = vec2(p0.delta.y, -p0.delta.x);
                        let dl1 = vec2(p1.delta.y, -p1.delta.x);

                        if p1.flags.contains(PointFlags::LEFT) {
                            verts.push(vert(p1.pos + (p1.dm * woff), point2(0.5, 1.0)));
                        } else {
                            verts.push(vert(p1.pos + (dl0 * woff), UV_HALF_1));
                            verts.push(vert(p1.pos + (dl1 * woff), UV_HALF_1));
                        }
                    } else {
                        verts.push(vert(p1.pos + (p1.dm * woff), UV_HALF_1));
                    }
                };

                process_pair(
                    pts.last().unwrap(),
                    pts.first().unwrap(),
                    &mut self.cache.verts,
                );
                for i in 1..pts.len() {
                    process_pair(&pts[i - 1], &pts[i], &mut self.cache.verts);
                }
            } else {
                for p in pts.iter() {
                    self.cache.verts.push(vert(p.pos, point2(0.5, 1.0)));
                }
            }

            path.num_fill_verts = self.cache.verts.len() - path.fill_vert_start_index;

            // Calculate fringe
            if fringe {
                let (lw, lu) = if convex {
                    // Create only half a fringe for convex shapes so that
                    // the shape can be rendered without stenciling.
                    (
                        // This should generate the same vertex as fill inset above.
                        woff, // Set outline fade at middle.
                        0.5,
                    )
                } else {
                    (w + woff, 0.0)
                };

                let rw = w - woff;
                let ru = 1.0;

                path.stroke_vert_start_index = self.cache.verts.len();

                let process_pair = |p0: &PathPoint, p1: &PathPoint, verts: &mut Vec<Vertex>| {
                    if p1
                        .flags
                        .intersects(PointFlags::BEVEL | PointFlags::INNER_BEVEL)
                    {
                        bevel_join(verts, p0, p1, lw, rw, lu, ru);
                    } else {
                        verts.push(vert(p1.pos + (p1.dm * lw), point2(lu, 1.0)));
                        verts.push(vert(p1.pos - (p1.dm * rw), point2(ru, 1.0)));
                    }
                };

                process_pair(
                    pts.last().unwrap(),
                    pts.first().unwrap(),
                    &mut self.cache.verts,
                );
                for i in 1..pts.len() {
                    process_pair(&pts[i - 1], &pts[i], &mut self.cache.verts);
                }
            } else {
                path.num_stroke_verts = 0;
            }
        }
    }

    #[inline]
    fn state(&self) -> &State {
        self.states.last()
    }

    #[inline]
    fn state_mut(&mut self) -> &mut State {
        self.states.last_mut()
    }
}

fn normalize(p: &mut Vector2D<f32>) -> f32 {
    let d = ((p.x * p.x) + (p.y * p.y)).sqrt();
    if d > 1e-6 {
        let id = 1.0 / d;
        p.x *= id;
        p.y *= id;
    }
    d
}

fn point_approx_equals(p0: Point2D<f32>, p1: Point2D<f32>, tol: f32) -> bool {
    p0.distance_to(p1) < tol * tol
}

fn dist_point_seg(p0: Point2D<f32>, p1: Point2D<f32>, q: Point2D<f32>) -> f32 {
    let pq = q - p1;
    let mut d0 = p0 - p1;

    let d = pq.x * pq.x + pq.y * pq.y;
    let mut t = pq.x * d0.x + pq.y * d0.y;

    if d > 0.0 {
        t /= d;
    }
    t = t.clamp(0.0, 1.0);

    d0.x = p1.x + t * pq.x - p0.x;
    d0.y = p1.y + t * pq.y - p0.y;

    d0.x * d0.x + d0.y * d0.y
}

fn transform_average_scale(t: &Transform2D<f32>) -> f32 {
    let sx = (t.m11 * t.m11 + t.m21 * t.m21).sqrt();
    let sy = (t.m12 * t.m12 + t.m22 * t.m22).sqrt();
    (sx + sy) * 0.5
}

fn tri_area2(a: Point2D<f32>, b: Point2D<f32>, c: Point2D<f32>) -> f32 {
    let ab = b - a;
    let ac = c - a;
    ac.x * ab.y - ab.x * ac.y
}

fn poly_area(pts: &[PathPoint]) -> f32 {
    let mut area = 0.0;
    for i in 2..pts.len() {
        area += tri_area2(pts[0].pos, pts[i - 1].pos, pts[i].pos);
    }
    area * 0.5
}

fn curve_divs(r: f32, arc: f32, tol: f32) -> usize {
    let da = (r / (r + tol)).cos() * 2.0;
    2.max((arc / da).ceil() as usize)
}

fn choose_bevel(
    bevel: bool,
    p0: &PathPoint,
    p1: &PathPoint,
    w: f32,
) -> (Point2D<f32>, Point2D<f32>) {
    if bevel {
        (
            point2(p1.pos.x + p0.delta.y * w, p1.pos.y - p0.delta.x * w),
            point2(p1.pos.x + p1.delta.y * w, p1.pos.y - p1.delta.x * w),
        )
    } else {
        let p = p1.pos + (p1.dm * w);
        (p, p)
    }
}

const UV_HALF_1: Point2D<f32> = point2(0.5, 1.0);

fn round_join(
    dst: &mut Vec<Vertex>,
    p0: &PathPoint,
    p1: &PathPoint,
    lw: f32,
    rw: f32,
    lu: f32,
    ru: f32,
    ncap: usize,
) {
    let dl0 = vec2(p0.delta.y, -p0.delta.x);
    let dl1 = vec2(p1.delta.y, -p1.delta.x);

    let uv_l1 = point2(lu, 1.0);
    let uv_r1 = point2(ru, 1.0);

    if p1.flags.contains(PointFlags::LEFT) {
        let (l0, l1) = choose_bevel(p1.flags.contains(PointFlags::INNER_BEVEL), p0, p1, lw);
        let a0 = (-dl0.y).atan2(-dl0.x);
        let mut a1 = (-dl1.y).atan2(-dl1.x);
        if a1 > a0 {
            a1 -= TAU;
        }

        dst.push(vert(l0, uv_l1));
        dst.push(vert(p1.pos - (dl0 * rw), uv_r1));

        let n = ((((a0 - a1) * (1.0 / PI)) * ncap as f32).ceil() as usize).clamp(2, ncap);
        for i in 0..n {
            let u = i as f32 / (n - 1) as f32;
            let a = a0 + u * (a1 - a0);
            let r = point2(p1.pos.x + a.cos() * rw, p1.pos.y + a.sin() * rw);

            dst.push(vert(p1.pos, UV_HALF_1));
            dst.push(vert(r, uv_r1));
        }

        dst.push(vert(l1, uv_l1));
        dst.push(vert(p1.pos - (dl1 * rw), uv_r1));
    } else {
        let (r0, r1) = choose_bevel(p1.flags.contains(PointFlags::INNER_BEVEL), p0, p1, -rw);
        let a0 = dl0.y.atan2(dl0.x);
        let mut a1 = dl1.y.atan2(dl1.x);
        if a1 < a0 {
            a1 += TAU
        };

        dst.push(vert(p1.pos + (dl0 * rw), uv_l1));
        dst.push(vert(r0, uv_r1));

        let n = ((((a1 - a0) * (1.0 / PI)) * ncap as f32).ceil() as usize).clamp(2, ncap);
        for i in 0..n {
            let u = i as f32 / (n - 1) as f32;
            let a = a0 + u * (a1 - a0);
            let l = point2(p1.pos.x + a.cos() * lw, p1.pos.y + a.sin() * lw);

            dst.push(vert(l, uv_l1));
            dst.push(vert(p1.pos, UV_HALF_1));
        }

        dst.push(vert(p1.pos + (dl1 * rw), uv_l1));
        dst.push(vert(r1, uv_r1));
    }
}

fn bevel_join(
    dst: &mut Vec<Vertex>,
    p0: &PathPoint,
    p1: &PathPoint,
    lw: f32,
    rw: f32,
    lu: f32,
    ru: f32,
) {
    let dl0 = vec2(p0.delta.y, -p0.delta.x);
    let dl1 = vec2(p1.delta.y, -p1.delta.x);

    let uv_l1 = point2(lu, 1.0);
    let uv_r1 = point2(ru, 1.0);

    if p1.flags.contains(PointFlags::LEFT) {
        let (l0, l1) = choose_bevel(p1.flags.contains(PointFlags::INNER_BEVEL), p0, p1, lw);

        dst.push(vert(l0, uv_l1));
        dst.push(vert(p1.pos - (dl0 * rw), uv_r1));

        if p1.flags.contains(PointFlags::BEVEL) {
            dst.push(vert(l0, uv_l1));
            dst.push(vert(p1.pos - (dl0 * rw), uv_r1));

            dst.push(vert(l1, uv_l1));
            dst.push(vert(p1.pos - (dl1 * rw), uv_r1));
        } else {
            let r0 = p1.pos - (p1.dm * rw);

            dst.push(vert(p1.pos, UV_HALF_1));
            dst.push(vert(p1.pos - (dl0 * rw), uv_r1));

            dst.push(vert(r0, uv_r1));
            dst.push(vert(r0, uv_r1));

            dst.push(vert(p1.pos, UV_HALF_1));
            dst.push(vert(p1.pos - (dl1 * rw), uv_r1));
        }

        dst.push(vert(l1, uv_l1));
        dst.push(vert(p1.pos - (dl1 * rw), uv_r1));
    } else {
        let (r0, r1) = choose_bevel(p1.flags.contains(PointFlags::INNER_BEVEL), p0, p1, -rw);

        dst.push(vert(p1.pos + (dl0 * lw), uv_l1));
        dst.push(vert(r0, uv_r1));

        if p1.flags.contains(PointFlags::BEVEL) {
            dst.push(vert(p1.pos + (dl0 * lw), uv_l1));
            dst.push(vert(r0, uv_r1));

            dst.push(vert(p1.pos + (dl1 * lw), uv_l1));
            dst.push(vert(r1, uv_r1));
        } else {
            let l0 = p1.pos + (p1.dm * lw);

            dst.push(vert(p1.pos + (dl0 * lw), uv_l1));
            dst.push(vert(p1.pos, UV_HALF_1));

            dst.push(vert(l0, uv_l1));
            dst.push(vert(l0, uv_l1));

            dst.push(vert(p1.pos + (dl1 * lw), uv_l1));
            dst.push(vert(p1.pos, UV_HALF_1));
        }

        dst.push(vert(p1.pos + (dl1 * lw), uv_l1));
        dst.push(vert(r1, uv_r1));
    }
}

fn butt_cap_start(
    dst: &mut Vec<Vertex>,
    p: &PathPoint,
    delta: Vector2D<f32>,
    w: f32,
    d: f32,
    aa: f32,
    u0: f32,
    u1: f32,
) {
    let p1 = p.pos - (delta * d);
    let dl = vec2(delta.y, -delta.x);

    dst.push(vert(p1 + (dl * w) - (delta * aa), point2(u0, 0.0)));
    dst.push(vert(p1 - (dl * w) - (delta * aa), point2(u1, 0.0)));
    dst.push(vert(p1 + (dl * w), point2(u0, 1.0)));
    dst.push(vert(p1 - (dl * w), point2(u1, 1.0)));
}

fn butt_cap_end(
    dst: &mut Vec<Vertex>,
    p: &PathPoint,
    delta: Vector2D<f32>,
    w: f32,
    d: f32,
    aa: f32,
    u0: f32,
    u1: f32,
) {
    let p1 = p.pos + (delta * d);
    let dl = vec2(delta.y, -delta.x);

    dst.push(vert(p1 + (dl * w), point2(u0, 1.0)));
    dst.push(vert(p1 - (dl * w), point2(u1, 1.0)));
    dst.push(vert(p1 + (dl * w) + (delta * aa), point2(u0, 0.0)));
    dst.push(vert(p1 - (dl * w) + (delta * aa), point2(u1, 0.0)));
}

fn round_cap_start(
    dst: &mut Vec<Vertex>,
    p: &PathPoint,
    delta: Vector2D<f32>,
    w: f32,
    ncap: usize,
    u0: f32,
    u1: f32,
) {
    if ncap < 2 {
        return;
    }

    let dl = vec2(delta.y, -delta.x);

    for i in 0..ncap {
        let a = i as f32 / (ncap - 1) as f32 * PI;
        let ax = a.cos() * w;
        let ay = a.sin() * w;

        dst.push(vert(p.pos - (dl * ax) - (delta * ay), point2(u0, 1.0)));
        dst.push(vert(p.pos, UV_HALF_1));
    }

    dst.push(vert(p.pos + (dl * w), point2(u0, 1.0)));
    dst.push(vert(p.pos - (dl * w), point2(u1, 1.0)));
}

fn round_cap_end(
    dst: &mut Vec<Vertex>,
    p: &PathPoint,
    delta: Vector2D<f32>,
    w: f32,
    ncap: usize,
    u0: f32,
    u1: f32,
) {
    if ncap < 2 {
        return;
    }

    let dl = vec2(delta.y, -delta.x);

    dst.push(vert(p.pos + (dl * w), point2(u0, 1.0)));
    dst.push(vert(p.pos - (dl * w), point2(u1, 1.0)));

    for i in 0..ncap {
        let a = i as f32 / (ncap - 1) as f32 * PI;
        let ax = a.cos() * w;
        let ay = a.sin() * w;

        dst.push(vert(p.pos, UV_HALF_1));
        dst.push(vert(p.pos - (dl * ax) + (delta * ay), point2(u0, 1.0)));
    }
}

fn sign_of(x: f32) -> f32 {
    if x >= 0.0 {
        1.0
    } else {
        -1.0
    }
}
