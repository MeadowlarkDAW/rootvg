use std::num::NonZeroUsize;

use euclid::{
    default::{Point2D, Rect, Size2D, Transform2D, Vector2D},
    num::Ceil,
    Angle,
};
use vec1::Vec1;

use crate::{
    Align, BlendFactor, Color, CompositeOperation, CompositeOperationState, LineCap, Paint, Path,
    Scissor, Vertex, Winding,
};

const INIT_FONTIMAGE_SIZE: usize = 512;
const MAX_FONTIMAGE_SIZE: usize = 2048;
const MAX_FONTIMAGES: usize = 4;

const INIT_COMMANDS_SIZE: usize = 64;
const INIT_POINTS_SIZE: usize = 128;
const INIT_PATHS_SIZE: usize = 16;
const INIT_VERTS_SIZE: usize = 256;
const INIT_STATES: usize = 64;

/// Length proportional to radius of a cubic bezier handle for 90deg arcs
const KAPPA90: f32 = 0.5522847493;

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
        p: Point2D<f32>,
        h1: Point2D<f32>,
        h2: Point2D<f32>,
    },
    Close,
    Winding,
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
struct Point {
    p: Point2D<f32>,
    d: Vector2D<f32>,
    len: f32,
    dm: Vector2D<f32>,
    flags: PointFlags,
}

impl Point {
    fn new(p: Point2D<f32>, flags: PointFlags) -> Self {
        Self {
            p,
            d: Vector2D::zero(),
            len: 0.0,
            dm: Vector2D::zero(),
            flags,
        }
    }
}

struct PathCache {
    points: Vec<Point>,
    paths: Vec<Path>,
    verts: Vec<Vertex>,
    bounds: Rect<f32>,
}

impl PathCache {
    fn new() -> Self {
        Self {
            points: Vec::with_capacity(INIT_POINTS_SIZE),
            paths: Vec::with_capacity(INIT_PATHS_SIZE),
            verts: Vec::with_capacity(INIT_VERTS_SIZE),
            bounds: Rect::default(),
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
    command_x: f32,
    command_y: f32,
    states: Vec1<State>,
    cache: PathCache,
    tess_to_l: f32,
    dist_to_l: f32,
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
            command_x: 0.0,
            command_y: 0.0,
            states: Vec1::with_capacity(State::default(), INIT_STATES),
            cache: PathCache::new(),
            tess_to_l: 0.0,
            dist_to_l: 0.0,
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
        self.tess_to_l = 0.25 / ratio;
        self.dist_to_l = 0.01 / ratio;
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

    pub fn translate(&mut self, v: Vector2D<f32>) {
        let state = self.state_mut();
        state.xform = state.xform.pre_translate(v);
    }

    pub fn rotate(&mut self, angle: Angle<f32>) {
        let state = self.state_mut();
        state.xform = state.xform.pre_rotate(angle);
    }

    pub fn skew_x(&mut self, angle: Angle<f32>) {
        let state = self.state_mut();
        state.xform = crate::math::transform_skew_x(angle).then(&state.xform);
    }

    pub fn skew_y(&mut self, angle: Angle<f32>) {
        let state = self.state_mut();
        state.xform = crate::math::transform_skew_y(angle).then(&state.xform);
    }

    pub fn scale(&mut self, x: f32, y: f32) {
        let state = self.state_mut();
        state.xform = state.xform.pre_scale(x, y);
    }

    pub fn current_transform(&self) -> &Transform2D<f32> {
        &self.state().xform
    }

    pub fn stroke_color(&mut self, color: Color) {
        self.state_mut().stroke = Paint::new(color);
    }

    pub fn stroke_paint(&mut self, paint: Paint) {
        let state = self.state_mut();
        state.stroke = paint;
        state.stroke.xform = state.stroke.xform.then(&state.xform);
    }

    pub fn fill_color(&mut self, color: Color) {
        self.state_mut().fill = Paint::new(color);
    }

    pub fn fill_paint(&mut self, paint: Paint) {
        let state = self.state_mut();
        state.fill = paint;
        state.fill.xform = state.fill.xform.then(&state.xform);
    }

    pub fn scissor(&mut self, mut rect: Rect<f32>) {
        let state = self.state_mut();

        rect.size.width = rect.size.width.max(0.0);
        rect.size.height = rect.size.height.max(0.0);

        state.scissor.xform = Transform2D::identity();
        state.scissor.xform.m31 = rect.min_x() + rect.width() * 0.5;
        state.scissor.xform.m32 = rect.min_y() + rect.height() * 0.5;
        state.scissor.xform = state.scissor.xform.then(&state.xform);

        state.scissor.extent = Size2D::new(rect.width() * 0.5, rect.height() * 0.5)
    }

    // TODO: Custom error
    pub fn intersect_scissor(&mut self, rect: Rect<f32>) -> Result<(), ()> {
        let state = self.state_mut();

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
            Point2D::new(p_xform.m31 - tex, p_xform.m32 - tey),
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

    fn append_commands(&mut self, commands: impl Iterator<Item = Command>) {
        let Self {
            states,
            commands: self_commands,
            ..
        } = self;
        let state = states.last_mut();

        for mut cmd in commands {
            match &mut cmd {
                Command::MoveTo(p) => {
                    *p = state.xform.transform_point(*p);
                }
                Command::LineTo(p) => {
                    *p = state.xform.transform_point(*p);
                }
                Command::BezierTo { p, h1, h2 } => {
                    *p = state.xform.transform_point(*p);
                    *h1 = state.xform.transform_point(*h1);
                    *h2 = state.xform.transform_point(*h2);
                }
                _ => {}
            }

            self_commands.push(cmd);
        }
    }

    fn clear_path_cache(&mut self) {
        self.cache.clear();
    }

    fn add_point(&mut self, p: Point2D<f32>, flags: PointFlags) {
        let Some(path) = self.cache.paths.last_mut() else {
            return;
        };

        if path.count > 0 {
            if let Some(prev_p) = self.cache.points.last_mut() {
                if point_approx_equals(prev_p.p, p, self.dist_to_l) {
                    prev_p.flags |= flags;
                    return;
                }
            }
        }

        self.cache.points.push(Point::new(p, flags));
        path.count += 1;
    }

    fn close_path(&mut self) {
        let Some(path) = self.cache.paths.last_mut() else {
            return;
        };

        path.closed = true;
    }

    fn path_winding(&mut self, winding: Winding) {
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

        if (d2 + d3) * (d2 + d3) < self.tess_to_l * (dp.x * dp.x + dp.y * dp.y) {
            self.add_point(p4, flags);
            return;
        }

        let p234 = (p23 + p34.to_vector()) * 0.5;
        let p1234 = (p123 + p234.to_vector()) * 0.5;

        self.tesselate_bezier(p1, p12, p123, p1234, level + 1, PointFlags::empty());
        self.tesselate_bezier(p1234, p234, p34, p4, level + 1, flags)
    }

    // static void nvg__flattenPaths(NVGcontext* ctx)

    #[inline]
    fn state(&self) -> &State {
        self.states.last()
    }

    #[inline]
    fn state_mut(&mut self) -> &mut State {
        self.states.last_mut()
    }
}

fn cross(dx0: f32, dy0: f32, dx1: f32, dy1: f32) -> f32 {
    (dx1 * dy0) - (dx0 * dy1)
}

fn normalize(x: &mut f32, y: &mut f32) -> f32 {
    let d = ((*x * *x) + (*y * *y)).sqrt();
    if d > 1e-6 {
        let id = 1.0 / d;
        *x *= id;
        *y *= id;
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

fn poly_area(pts: &[Point]) -> f32 {
    let mut area = 0.0;
    for i in 2..pts.len() {
        area += tri_area2(pts[0].p, pts[i - 1].p, pts[i].p);
    }
    area * 0.5
}

fn poly_revers(pts: &mut [Point]) {
    pts.reverse()
}

fn curve_divs(r: f32, arc: f32, tol: f32) -> usize {
    let da = (r / (r + tol)).cos() * 2.0;
    2.max((arc / da).ceil() as usize)
}

fn choose_bevel(bevel: bool, p0: &Point, p1: &Point, w: f32) -> (Point2D<f32>, Point2D<f32>) {
    if bevel {
        (
            Point2D::new(p1.p.x + p0.d.y * w, p1.p.y - p0.d.x * w),
            Point2D::new(p1.p.x + p1.d.y * w, p1.p.y - p1.d.x * w),
        )
    } else {
        let p = Point2D::new(p1.p.x + p1.dm.x * w, p1.p.y + p1.dm.y * w);
        (p, p)
    }
}

fn round_join(
    dst: &mut Vec<Vertex>,
    p0: &Point,
    p1: &Point,
    lw: f32,
    rw: f32,
    lu: f32,
    ru: f32,
    ncap: i32,
) {
    let dl0 = Point2D::new(p0.d.y, -p0.d.x);
    let dl1 = Point2D::new(p1.d.y, -p1.d.x);

    if p1.flags.contains(PointFlags::LEFT) {
        let (l0, l1) = choose_bevel(p1.flags.contains(PointFlags::INNER_BEVEL), p0, p1, lw);
        let a0 = (-dl0.y).atan2(-dl0.x);
        let mut a1 = (-dl1.y).atan2(-dl1.x);
        if a1 > a0 {
            a1 -= std::f32::consts::TAU;
        }

        dst.push(Vertex {
            x: l0.x,
            y: l0.y,
            u: lu,
            v: 1.0,
        });
        dst.push(Vertex {
            x: p1.p.x - dl0.x * rw,
            y: p1.p.y - dl0.y * rw,
            u: ru,
            v: 1.0,
        });

        let n = ((((a0 - a1) / std::f32::consts::PI) * ncap as f32).ceil() as i32).clamp(2, ncap);
        for i in 0..n {
            let u = i as f32 / (n - 1) as f32;
            let a = a0 + u * (a1 - a0);
            let rx = p1.p.x + a.cos() * rw;
            let ry = p1.p.y + a.sin() * rw;

            dst.push(Vertex {
                x: p1.p.x,
                y: p1.p.y,
                u: 0.5,
                v: 1.0,
            });
            dst.push(Vertex {
                x: rx,
                y: ry,
                u: ru,
                v: 1.0,
            });
        }

        dst.push(Vertex {
            x: l1.x,
            y: l1.y,
            u: lu,
            v: 1.0,
        });
        dst.push(Vertex {
            x: p1.p.x - dl1.x * rw,
            y: p1.p.y - dl1.y * rw,
            u: ru,
            v: 1.0,
        });
    } else {
        let (r0, r1) = choose_bevel(p1.flags.contains(PointFlags::INNER_BEVEL), p0, p1, -rw);
        let a0 = dl0.y.atan2(dl0.x);
        let mut a1 = dl1.y.atan2(dl1.x);
        if a1 < a0 {
            a1 += std::f32::consts::TAU
        };

        dst.push(Vertex {
            x: p1.p.x + dl0.x * rw,
            y: p1.p.y + dl0.y * rw,
            u: lu,
            v: 1.0,
        });
        dst.push(Vertex {
            x: r0.x,
            y: r0.y,
            u: ru,
            v: 1.0,
        });

        let n = ((((a1 - a0) / std::f32::consts::PI) * ncap as f32).ceil() as i32).clamp(2, ncap);
        for i in 0..n {
            let u = i as f32 / (n - 1) as f32;
            let a = a0 + u * (a1 - a0);
            let lx = p1.p.x + a.cos() * lw;
            let ly = p1.p.y + a.sin() * lw;

            dst.push(Vertex {
                x: lx,
                y: ly,
                u: lu,
                v: 1.0,
            });
            dst.push(Vertex {
                x: p1.p.x,
                y: p1.p.y,
                u: 0.5,
                v: 1.0,
            });
        }

        dst.push(Vertex {
            x: p1.p.x + dl1.x * rw,
            y: p1.p.y + dl1.y * rw,
            u: lu,
            v: 1.0,
        });
        dst.push(Vertex {
            x: r1.x,
            y: r1.y,
            u: ru,
            v: 1.0,
        });
    }
}

// static NVGvertex* nvg__bevelJoin

fn butt_cap_start(
    dst: &mut Vec<Vertex>,
    p: &Point,
    dt: Vector2D<f32>,
    w: f32,
    d: f32,
    aa: f32,
    u0: f32,
    u1: f32,
) {
    let p1 = p.p - (dt * d);
    let dl = Vector2D::new(dt.y, -dt.x);

    dst.push(Vertex {
        x: p1.x + dl.x * w - dt.x * aa,
        y: p1.y + dl.y * w - dt.y * aa,
        u: u0,
        v: 0.0,
    });
    dst.push(Vertex {
        x: p1.x - dl.x * w - dt.x * aa,
        y: p1.y - dl.y * w - dt.y * aa,
        u: u1,
        v: 0.0,
    });
    dst.push(Vertex {
        x: p1.x + dl.x * w,
        y: p1.y + dl.y * w,
        u: u0,
        v: 1.0,
    });
    dst.push(Vertex {
        x: p1.x - dl.x * w,
        y: p1.y - dl.y * w,
        u: u1,
        v: 1.0,
    });
}

fn butt_cap_end(
    dst: &mut Vec<Vertex>,
    p: &Point,
    dt: Vector2D<f32>,
    w: f32,
    d: f32,
    aa: f32,
    u0: f32,
    u1: f32,
) {
    let p1 = p.p + (dt * d);
    let dl = Vector2D::new(dt.y, -dt.x);

    dst.push(Vertex {
        x: p1.x + dl.x * w,
        y: p1.y + dl.y * w,
        u: u0,
        v: 1.0,
    });
    dst.push(Vertex {
        x: p1.x - dl.x * w,
        y: p1.y - dl.y * w,
        u: u1,
        v: 1.0,
    });
    dst.push(Vertex {
        x: p1.x + dl.x * w + dt.x * aa,
        y: p1.y + dl.y * w + dt.y * aa,
        u: u0,
        v: 0.0,
    });
    dst.push(Vertex {
        x: p1.x - dl.x * w + dt.x * aa,
        y: p1.y - dl.y * w + dt.y * aa,
        u: u1,
        v: 0.0,
    });
}