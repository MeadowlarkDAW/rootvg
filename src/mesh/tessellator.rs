use euclid::{
    default::{Point2D, Vector2D},
    point2, vec2,
};
use std::f32::consts::{PI, TAU};
use vec1::Vec1;

use crate::{vert, LineCap, LineJoin, Vertex, Winding};

use super::{Command, MeshBuilder};

const INIT_POINTS_SIZE: usize = 128;
const INIT_PATHS_SIZE: usize = 16;

pub(crate) struct Tessellator {
    scale_factor: f32,
    tess_tol: f32,
    dist_tol: f32,
    fringe_width: f32,

    points: Vec<PathPoint>,
    paths: Vec1<PathState>,
    bounds_tl: Point2D<f32>,
    bounds_br: Point2D<f32>,
}

impl Tessellator {
    pub(crate) fn new() -> Self {
        Self {
            tess_tol: 0.0,
            dist_tol: 0.0,
            scale_factor: 0.0,
            fringe_width: 0.0,

            points: Vec::with_capacity(INIT_POINTS_SIZE),
            paths: Vec1::with_capacity(PathState::new(0), INIT_PATHS_SIZE),
            bounds_tl: Point2D::default(),
            bounds_br: Point2D::default(),
        }
    }

    pub fn fringe_width(&self) -> f32 {
        self.fringe_width
    }

    pub fn set_scale_factor(&mut self, scale_factor: f32) {
        if self.scale_factor != scale_factor {
            self.scale_factor = scale_factor;
            self.fringe_width = scale_factor.recip();

            self.tess_tol = 0.25 * self.fringe_width;
            self.dist_tol = 0.01 * self.fringe_width;
        }
    }

    pub(super) fn tessellate(
        &mut self,
        builder: &MeshBuilder,
        stroke_verts: &mut Vec<Vertex>,
        fill_verts: &mut Vec<Vertex>,
        antialias: bool,
    ) {
        let stroke_width = f32::from_ne_bytes(builder.inner.stroke_width_bytes);

        if !builder.inner.fill && stroke_width <= 0.0 {
            return;
        }

        self.points.clear();
        self.paths.truncate(1).unwrap();
        *self.paths.first_mut() = PathState::new(0);

        // flatten
        for cmd in builder.inner.command_buffer.iter() {
            match cmd {
                Command::MoveTo(pos) => {
                    self.paths.push(PathState::new(self.points.len()));
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
                    if let Some(last_point) = self.points.last() {
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
                    self.close_path();
                }
                Command::Winding(winding) => {
                    self.set_path_winding(winding);
                }
            }
        }

        self.bounds_tl = point2(1e6, 1e6);
        self.bounds_br = point2(-1e6, -1e6);

        // Calculate the direction and length of line segments.
        for path in self.paths.iter_mut() {
            if path.num_points == 0 {
                continue;
            }

            let mut pts =
                &mut self.points[path.point_start_index..path.point_start_index + path.num_points];

            // If the first and last points are the same, remove the last, mark as closed path.
            if pts.len() > 1 {
                if super::point_approx_equals(
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
                if (path.winding == Winding::Solid && area < 0.0)
                    || (path.winding == Winding::Hole && area > 0.0)
                {
                    pts.reverse();
                }
            }

            let mut process_pt = |p: &mut PathPoint, next_pos: Point2D<f32>| {
                // Calculate segment direction and length
                p.delta = next_pos - p.pos;
                p.len = super::normalize(&mut p.delta);

                // Update bounds
                self.bounds_tl = self.bounds_tl.min(p.pos);
                self.bounds_br = self.bounds_br.max(p.pos);
            };

            let first_pos = pts.first().unwrap().pos;
            process_pt(pts.last_mut().unwrap(), first_pos);

            for i in 1..pts.len() {
                let next_pos = pts[i].pos;
                process_pt(&mut pts[i - 1], next_pos);
            }
        }

        let fringe_width = if antialias { self.fringe_width } else { 0.0 };

        let miter_limit = f32::from_ne_bytes(builder.inner.miter_limit_bytes).max(0.0);

        if builder.inner.fill {
            self.expand_fill(
                fill_verts,
                fringe_width,
                builder.inner.line_join,
                miter_limit,
            );
        }

        if stroke_width > 0.0 {
            self.expand_stroke(
                stroke_verts,
                stroke_width * 0.5,
                fringe_width,
                builder.inner.line_cap,
                builder.inner.line_join,
                miter_limit,
            )
        }
    }

    fn add_point(&mut self, pos: Point2D<f32>, flags: PointFlags) {
        let path = self.paths.last_mut();

        if path.num_points > 0 {
            if let Some(prev_p) = self.points.last_mut() {
                if super::point_approx_equals(prev_p.pos, pos, self.dist_tol) {
                    prev_p.flags |= flags;
                    return;
                }
            }
        }

        self.points.push(PathPoint::new(pos, flags));
        path.num_points += 1;
    }

    fn close_path(&mut self) {
        self.paths.last_mut().closed = true;
    }

    fn set_path_winding(&mut self, winding: Winding) {
        self.paths.last_mut().winding = winding;
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

    fn calculate_joins(&mut self, w: f32, line_join: LineJoin, miter_limit: f32) {
        let iw = if w > 0.0 { 1.0 / w } else { 0.0 };

        // Calculate which joins needs extra vertices to append, and gather vertex count.
        for path in self.paths.iter_mut() {
            if path.num_points == 0 {
                continue;
            }

            let pts =
                &mut self.points[path.point_start_index..path.point_start_index + path.num_points];

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
                        || line_join == LineJoin::Bevel
                        || line_join == LineJoin::Round
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
        verts: &mut Vec<Vertex>,
        w: f32,
        fringe: f32,
        line_cap: LineCap,
        line_join: LineJoin,
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
        for path in self.paths.iter() {
            if line_join == LineJoin::Round {
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

        verts.reserve(vert_capacity);

        for path in self.paths.iter_mut() {
            if path.num_points < 2 {
                continue;
            }

            let pts =
                &self.points[path.point_start_index..path.point_start_index + path.num_points];

            // Calculate fringe or stroke

            path.num_fill_verts = 0;
            path.stroke_vert_start_index = verts.len();

            let join_pt = |p0: &PathPoint, p1: &PathPoint, verts: &mut Vec<Vertex>| {
                if p1
                    .flags
                    .intersects(PointFlags::BEVEL | PointFlags::INNER_BEVEL)
                {
                    if line_join == LineJoin::Round {
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
                join_pt(pts.last().unwrap(), pts.first().unwrap(), verts);
                for i in 1..pts.len() {
                    join_pt(&pts[i - 1], &pts[i], verts);
                }

                // Loop it
                let first_vert_pos = verts[path.stroke_vert_start_index].pos;
                let second_vert_pos = verts[path.stroke_vert_start_index + 1].pos;
                verts.push(vert(first_vert_pos, point2(u0, 1.0)));
                verts.push(vert(second_vert_pos, point2(u1, 1.0)));
            } else {
                // Add start cap
                let p0 = &pts[0];
                let p1 = &pts[1];
                let mut delta = p1.pos - p0.pos;
                super::normalize(&mut delta);
                match line_cap {
                    LineCap::Butt => butt_cap_start(verts, p0, delta, w, -aa * 0.5, aa, u0, u1),
                    LineCap::Square => butt_cap_start(verts, p0, delta, w, w - aa, aa, u0, u1),
                    LineCap::Round => round_cap_start(verts, p0, delta, w, ncap, u0, u1),
                    _ => {}
                }

                // Join points
                for i in 1..(pts.len() - 1) {
                    join_pt(&pts[i - 1], &pts[i], verts);
                }

                // Add end cap
                let p0 = &pts[pts.len() - 2];
                let p1 = &pts[pts.len() - 1];
                let mut delta = p1.pos - p0.pos;
                super::normalize(&mut delta);
                match line_cap {
                    LineCap::Butt => butt_cap_end(verts, p0, delta, w, -aa * 0.5, aa, u0, u1),
                    LineCap::Square => butt_cap_end(verts, p0, delta, w, w - aa, aa, u0, u1),
                    LineCap::Round => round_cap_end(verts, p0, delta, w, ncap, u0, u1),
                    _ => {}
                }
            };

            path.num_stroke_verts = verts.len() - path.stroke_vert_start_index;
        }
    }

    fn expand_fill(
        &mut self,
        verts: &mut Vec<Vertex>,
        w: f32,
        line_join: LineJoin,
        miter_limit: f32,
    ) {
        let aa = self.fringe_width;
        let fringe = w > 0.0;

        self.calculate_joins(w, line_join, miter_limit);

        // Calculate max vertex usage
        let mut vert_capacity = 0;
        for path in self.paths.iter() {
            vert_capacity += path.num_points + path.num_bevels + 1;

            if fringe {
                // plus one for loop
                vert_capacity += (path.num_points + path.num_bevels * 5 + 1) * 2;
            }
        }

        verts.reserve(vert_capacity);

        for path in self.paths.iter_mut() {
            if path.num_points < 2 {
                continue;
            }

            let pts =
                &self.points[path.point_start_index..path.point_start_index + path.num_points];

            // Calculate shape vertices
            let woff = 0.5 * aa;
            path.fill_vert_start_index = verts.len();

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

                process_pair(pts.last().unwrap(), pts.first().unwrap(), verts);
                for i in 1..pts.len() {
                    process_pair(&pts[i - 1], &pts[i], verts);
                }
            } else {
                for p in pts.iter() {
                    verts.push(vert(p.pos, point2(0.5, 1.0)));
                }
            }

            path.num_fill_verts = verts.len() - path.fill_vert_start_index;

            // Calculate fringe
            if fringe {
                let (lw, lu) = if path.convex {
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

                path.stroke_vert_start_index = verts.len();

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

                process_pair(pts.last().unwrap(), pts.first().unwrap(), verts);
                for i in 1..pts.len() {
                    process_pair(&pts[i - 1], &pts[i], verts);
                }
            } else {
                path.num_stroke_verts = 0;
            }
        }
    }
}

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

#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct PathState {
    point_start_index: usize,
    num_points: usize,
    closed: bool,
    num_bevels: usize,
    fill_vert_start_index: usize,
    num_fill_verts: usize,
    stroke_vert_start_index: usize,
    num_stroke_verts: usize,
    winding: Winding,
    convex: bool,
}

impl PathState {
    pub(crate) fn new(point_start_index: usize) -> Self {
        Self {
            point_start_index,
            num_points: 0,
            closed: false,
            num_bevels: 0,
            fill_vert_start_index: 0,
            num_fill_verts: 0,
            stroke_vert_start_index: 0,
            num_stroke_verts: 0,
            winding: Winding::Solid,
            convex: false,
        }
    }
}

/*
fn transform_average_scale(t: &Transform2D<f32>) -> f32 {
    let sx = (t.m11 * t.m11 + t.m21 * t.m21).sqrt();
    let sy = (t.m12 * t.m12 + t.m22 * t.m22).sqrt();
    (sx + sy) * 0.5
}
*/

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
