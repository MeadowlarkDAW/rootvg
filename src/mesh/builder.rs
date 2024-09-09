use euclid::{
    default::{Point2D, Rect, Transform2D, Vector2D},
    point2, vec2, Angle,
};
use std::f32::consts::{PI, TAU};
use std::hash::Hash;

use crate::{LineCap, LineJoin, Winding};

use super::{PackedCommandBuffer, DEFAULT_MITER_LIMIT, KAPPA90, ONE_MINUS_KAPPA90};

#[derive(Clone, PartialEq, Eq, Hash)]
pub(super) struct MeshBuilderInner {
    pub command_buffer: PackedCommandBuffer,
    pub stroke_width_bytes: [u8; 4],
    pub miter_limit_bytes: [u8; 4],
    pub line_join: LineJoin,
    pub line_cap: LineCap,
    pub fill: bool,
    pub antialias: bool,
}

impl MeshBuilderInner {
    fn clear(&mut self) {
        self.command_buffer.data.clear();
        self.stroke_width_bytes = 0.0_f32.to_ne_bytes();
        self.miter_limit_bytes = DEFAULT_MITER_LIMIT.to_ne_bytes();
        self.fill = true;
        self.antialias = true;
    }
}

pub struct MeshBuilder {
    pub(super) inner: MeshBuilderInner,
    pos: Point2D<f32>,
    xform: Option<Transform2D<f32>>,
    dist_tol: f32,
}

impl MeshBuilder {
    pub(crate) fn new() -> Self {
        Self {
            inner: MeshBuilderInner {
                command_buffer: PackedCommandBuffer::new(),
                stroke_width_bytes: 0.0_f32.to_ne_bytes(),
                miter_limit_bytes: DEFAULT_MITER_LIMIT.to_ne_bytes(),
                line_join: LineJoin::default(),
                line_cap: LineCap::default(),
                fill: true,
                antialias: true,
            },
            pos: Point2D::default(),
            xform: None,
            dist_tol: 0.0,
        }
    }

    pub(crate) fn set_scale_factor(&mut self, scale_factor: f32) {
        self.dist_tol = 0.01 / scale_factor;
    }

    pub fn clear(&mut self) {
        self.inner.clear();
        self.pos = Point2D::default();
        self.xform = None;
    }

    pub fn move_to(&mut self, pos: impl Into<Point2D<f32>>) {
        let mut pos: Point2D<f32> = pos.into();
        if let Some(xform) = self.xform {
            pos = xform.transform_point(pos);
        }

        self.inner.command_buffer.move_to(pos);
        self.pos = pos;
    }

    pub fn line_to(&mut self, pos: impl Into<Point2D<f32>>) {
        let mut pos: Point2D<f32> = pos.into();
        if let Some(xform) = self.xform {
            pos = xform.transform_point(pos);
        }

        self.inner.command_buffer.line_to(pos);
        self.pos = pos;
    }

    pub fn bezier_to(
        &mut self,
        pos: impl Into<Point2D<f32>>,
        h1_pos: impl Into<Point2D<f32>>,
        h2_pos: impl Into<Point2D<f32>>,
    ) {
        let mut pos: Point2D<f32> = pos.into();
        let mut h1_pos: Point2D<f32> = h1_pos.into();
        let mut h2_pos: Point2D<f32> = h2_pos.into();

        if let Some(xform) = self.xform {
            pos = xform.transform_point(pos);
            h1_pos = xform.transform_point(h1_pos);
            h2_pos = xform.transform_point(h2_pos);
        }

        self.inner.command_buffer.bezier_to(pos, h1_pos, h2_pos);
        self.pos = pos;
    }

    pub fn close_path(&mut self) {
        self.inner.command_buffer.close_path();
    }

    pub fn path_winding(&mut self, winding: Winding) {
        self.inner.command_buffer.winding(winding);
    }

    pub fn quad_to(&mut self, pos: impl Into<Point2D<f32>>, c: impl Into<Point2D<f32>>) {
        let pos: Point2D<f32> = pos.into();
        let c: Point2D<f32> = c.into();

        self.bezier_to(
            pos,
            self.pos + ((c - self.pos) * (2.0 / 3.0)),
            pos + ((c - pos) * (2.0 / 3.0)),
        );
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

    pub fn arc_to(
        &mut self,
        p1: impl Into<Point2D<f32>>,
        p2: impl Into<Point2D<f32>>,
        radius: f32,
    ) {
        let p1: Point2D<f32> = p1.into();
        let p2: Point2D<f32> = p2.into();

        let p0 = self.pos;

        // Handle degenerate cases
        if super::point_approx_equals(p0, p1, self.dist_tol)
            || super::point_approx_equals(p1, p2, self.dist_tol)
            || super::dist_point_seg(p1, p0, p2) < self.dist_tol * self.dist_tol
            || radius < self.dist_tol
        {
            self.line_to(p1);
            return;
        }

        // Calculate tangential circle to lines (x0,y0)-(x1,y1) and (x1,y1)-(x2,y2)
        let mut d0 = p0 - p1;
        let mut d1 = p2 - p1;
        super::normalize(&mut d0);
        super::normalize(&mut d1);
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
            (center, a0, a1, Winding::Hole)
        } else {
            let center = point2(
                p1.x + d0.x * d - d0.y * radius,
                p1.y + d0.y * d + d0.x * radius,
            );
            let a0 = (-d0.x).atan2(d0.y);
            let a1 = d1.x.atan2(-d1.y);
            (center, a0, a1, Winding::Solid)
        };

        self.arc(center, Angle::radians(a0), Angle::radians(a1), radius, dir);
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
        if dir == Winding::Hole {
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
        if dir == Winding::Solid {
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
                if join != LineCap::Butt {
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

    pub fn transform(&mut self, t: impl Into<Transform2D<f32>>) {
        let t: Transform2D<f32> = t.into();

        if let Some(xform) = &mut self.xform {
            *xform = t.then(xform);
        } else {
            self.xform = Some(t);
        }

        self.xform = Some(t);
    }

    pub fn reset_transform(&mut self) {
        self.xform = None;
    }

    pub fn translate(&mut self, v: impl Into<Vector2D<f32>>) {
        let v: Vector2D<f32> = v.into();

        if let Some(xform) = &mut self.xform {
            *xform = xform.pre_translate(v);
        } else {
            self.xform = Some(Transform2D::translation(v.x, v.y));
        }
    }

    pub fn rotate(&mut self, angle: impl Into<Angle<f32>>) {
        let angle: Angle<f32> = angle.into();

        if let Some(xform) = &mut self.xform {
            *xform = xform.pre_rotate(angle);
        } else {
            self.xform = Some(Transform2D::rotation(angle));
        }
    }

    pub fn scale(&mut self, x: f32, y: f32) {
        if let Some(xform) = &mut self.xform {
            *xform = xform.pre_scale(x, y);
        } else {
            self.xform = Some(Transform2D::scale(x, y));
        }
    }

    pub fn skew_x(&mut self, angle: impl Into<Angle<f32>>) {
        let angle: Angle<f32> = angle.into();

        if let Some(xform) = &mut self.xform {
            *xform = crate::math::transform_skew_x(angle).then(&xform)
        } else {
            self.xform = Some(crate::math::transform_skew_x(angle));
        }
    }

    pub fn skew_y(&mut self, angle: impl Into<Angle<f32>>) {
        let angle: Angle<f32> = angle.into();

        if let Some(xform) = &mut self.xform {
            *xform = crate::math::transform_skew_y(angle).then(&xform)
        } else {
            self.xform = Some(crate::math::transform_skew_y(angle));
        }
    }

    pub fn current_transform(&self) -> Transform2D<f32> {
        self.xform.unwrap_or(Transform2D::default())
    }

    pub fn line_join(&mut self, line_join: LineJoin) {
        self.inner.line_join = line_join;
    }

    pub fn line_cap(&mut self, line_cap: LineCap) {
        self.inner.line_cap = line_cap;
    }

    pub fn antialias(&mut self, antialias: bool) {
        self.inner.antialias = antialias;
    }

    pub fn stroke_width(&mut self, stroke_width: f32) {
        self.inner.stroke_width_bytes = stroke_width.to_ne_bytes();
    }

    pub fn fill(&mut self, fill: bool) {
        self.inner.fill = fill;
    }

    pub fn miter_limit(&mut self, miter_limit: f32) {
        self.inner.miter_limit_bytes = miter_limit.to_ne_bytes();
    }
}

fn sign_of(x: f32) -> f32 {
    if x >= 0.0 {
        1.0
    } else {
        -1.0
    }
}
