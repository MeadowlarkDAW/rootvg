use euclid::{
    default::{Point2D, Rect, Transform2D, Vector2D},
    point2, vec2, Angle,
};
use std::f32::consts::{PI, TAU};

use crate::{paint::MeshOpts, Context, LineCap, Winding};

use super::{
    cache::{CachedMeshID, UncachedMeshID},
    KAPPA90, ONE_MINUS_KAPPA90,
};

pub struct MeshBuilder<'a> {
    r: &'a mut Context,
    pos: Point2D<f32>,
    xform: Option<Transform2D<f32>>,
}

impl<'a> MeshBuilder<'a> {
    pub(crate) fn new(r: &'a mut Context) -> Self {
        r.mesh_cache_key.reset();

        let xform = r.state().xform;

        let pos = if let Some(xform) = &xform {
            xform.transform_point(Point2D::zero())
        } else {
            Point2D::zero()
        };

        Self { r, xform, pos }
    }

    pub fn move_to(mut self, pos: impl Into<Point2D<f32>>) -> Self {
        let mut pos: Point2D<f32> = pos.into();
        if let Some(xform) = self.xform {
            pos = xform.transform_point(pos);
        }

        self.r.mesh_cache_key.command_buffer.move_to(pos);
        self.pos = pos;

        self
    }

    pub fn line_to(mut self, pos: impl Into<Point2D<f32>>) -> Self {
        let mut pos: Point2D<f32> = pos.into();
        if let Some(xform) = self.xform {
            pos = xform.transform_point(pos);
        }

        self.r.mesh_cache_key.command_buffer.line_to(pos);
        self.pos = pos;

        self
    }

    pub fn bezier_to(
        mut self,
        pos: impl Into<Point2D<f32>>,
        h1_pos: impl Into<Point2D<f32>>,
        h2_pos: impl Into<Point2D<f32>>,
    ) -> Self {
        let mut pos: Point2D<f32> = pos.into();
        let mut h1_pos: Point2D<f32> = h1_pos.into();
        let mut h2_pos: Point2D<f32> = h2_pos.into();

        if let Some(xform) = self.xform {
            pos = xform.transform_point(pos);
            h1_pos = xform.transform_point(h1_pos);
            h2_pos = xform.transform_point(h2_pos);
        }

        self.r
            .mesh_cache_key
            .command_buffer
            .bezier_to(pos, h1_pos, h2_pos);
        self.pos = pos;

        self
    }

    pub fn close_path(self) -> Self {
        self.r.mesh_cache_key.command_buffer.close_path();
        self
    }

    pub fn path_winding(self, winding: Winding) -> Self {
        self.r.mesh_cache_key.command_buffer.winding(winding);
        self
    }

    pub fn quad_to(self, pos: impl Into<Point2D<f32>>, c: impl Into<Point2D<f32>>) -> Self {
        let pos: Point2D<f32> = pos.into();
        let c: Point2D<f32> = c.into();

        let self_pos = self.pos;

        self.bezier_to(
            pos,
            self_pos + ((c - self_pos) * (2.0 / 3.0)),
            pos + ((c - pos) * (2.0 / 3.0)),
        )
    }

    pub fn rect(self, rect: impl Into<Rect<f32>>) -> Self {
        let rect: Rect<f32> = rect.into();

        self.move_to(rect.origin)
            .line_to(point2(rect.min_x(), rect.max_y()))
            .line_to(rect.max())
            .line_to(point2(rect.max_x(), rect.min_y()))
            .close_path()
    }

    pub fn rounded_rect(self, rect: impl Into<Rect<f32>>, radius: f32) -> Self {
        self.rounded_rect_varying(rect, radius, radius, radius, radius)
    }

    pub fn rounded_rect_varying(
        self,
        rect: impl Into<Rect<f32>>,
        rad_top_left: f32,
        rad_top_right: f32,
        rad_bottom_right: f32,
        rad_bottom_left: f32,
    ) -> Self {
        let rect: Rect<f32> = rect.into();

        if rad_top_left < 0.1
            && rad_top_right < 0.1
            && rad_bottom_right < 0.1
            && rad_bottom_left < 0.1
        {
            self.rect(rect)
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

            self.move_to(point2(rect.min_x(), rect.min_y() + tl.y))
                .line_to(point2(rect.min_x(), rect.max_y() - bl.y))
                .bezier_to(
                    point2(rect.min_x() + bl.x, rect.max_y()),
                    point2(rect.min_x(), rect.max_y() - bl.y * ONE_MINUS_KAPPA90),
                    point2(rect.min_x() + bl.x * ONE_MINUS_KAPPA90, rect.max_y()),
                )
                .line_to(point2(rect.max_x() - br.x, rect.max_y()))
                .bezier_to(
                    point2(rect.max_x(), rect.max_y() - br.y),
                    point2(rect.max_x() - br.x * ONE_MINUS_KAPPA90, rect.max_y()),
                    point2(rect.max_x(), rect.max_y() - br.y * ONE_MINUS_KAPPA90),
                )
                .line_to(point2(rect.max_x(), rect.min_y() + tr.y))
                .bezier_to(
                    point2(rect.max_x() - tr.x, rect.min_y()),
                    point2(rect.max_x(), rect.min_y() + tr.y * ONE_MINUS_KAPPA90),
                    point2(rect.max_x() - tr.x * ONE_MINUS_KAPPA90, rect.min_y()),
                )
                .line_to(point2(rect.min_x() + tl.x, rect.min_y()))
                .bezier_to(
                    point2(rect.min_x(), rect.min_y() + tl.y),
                    point2(rect.min_x() + tl.x * ONE_MINUS_KAPPA90, rect.min_y()),
                    point2(rect.min_x(), rect.min_y() + tl.y * ONE_MINUS_KAPPA90),
                )
                .close_path()
        }
    }

    pub fn arc_to(
        self,
        p1: impl Into<Point2D<f32>>,
        p2: impl Into<Point2D<f32>>,
        radius: f32,
    ) -> Self {
        let p1: Point2D<f32> = p1.into();
        let p2: Point2D<f32> = p2.into();

        let p0 = self.pos;

        // Handle degenerate cases
        if super::point_approx_equals(p0, p1, self.r.dist_tol)
            || super::point_approx_equals(p1, p2, self.r.dist_tol)
            || super::dist_point_seg(p1, p0, p2) < self.r.dist_tol * self.r.dist_tol
            || radius < self.r.dist_tol
        {
            return self.line_to(p1);
        }

        // Calculate tangential circle to lines (x0,y0)-(x1,y1) and (x1,y1)-(x2,y2)
        let mut d0 = p0 - p1;
        let mut d1 = p2 - p1;
        super::normalize(&mut d0);
        super::normalize(&mut d1);
        let a = (d0.x * d1.x + d0.y * d1.y).cos();
        let d = radius / (a * 0.5).tan();

        if d > 10_000.0 {
            return self.line_to(p1);
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

        self.arc(center, Angle::radians(a0), Angle::radians(a1), radius, dir)
    }

    pub fn arc(
        self,
        center: impl Into<Point2D<f32>>,
        angle_0: impl Into<Angle<f32>>,
        angle_1: impl Into<Angle<f32>>,
        radius: f32,
        dir: Winding,
    ) -> Self {
        self.barc(center, angle_0, angle_1, radius, dir, LineCap::Round)
    }

    pub fn barc(
        mut self,
        center: impl Into<Point2D<f32>>,
        angle_0: impl Into<Angle<f32>>,
        angle_1: impl Into<Angle<f32>>,
        radius: f32,
        dir: Winding,
        join: LineCap,
    ) -> Self {
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
                    self = self.line_to(pos);
                } else {
                    self = self.move_to(pos);
                }
            } else {
                self = self.bezier_to(pos, prev_pos + prev_tanv, pos - tanv);
            }

            prev_pos = pos;
            prev_tanv = tanv;
        }

        self
    }

    pub fn circle(self, center: impl Into<Point2D<f32>>, radius: f32) -> Self {
        self.ellipse(center, radius, radius)
    }

    pub fn ellipse(self, center: impl Into<Point2D<f32>>, rx: f32, ry: f32) -> Self {
        let center: Point2D<f32> = center.into();

        self.move_to(point2(center.x - rx, center.y))
            .bezier_to(
                point2(center.x, center.y + ry),
                point2(center.x - rx, center.y + ry * KAPPA90),
                point2(center.x - rx * KAPPA90, center.y + ry),
            )
            .bezier_to(
                point2(center.x + rx, center.y),
                point2(center.x + rx * KAPPA90, center.y + ry),
                point2(center.x + rx, center.y + ry * KAPPA90),
            )
            .bezier_to(
                point2(center.x, center.y - ry),
                point2(center.x + rx, center.y - ry * KAPPA90),
                point2(center.x + rx * KAPPA90, center.y - ry),
            )
            .bezier_to(
                point2(center.x - rx, center.y),
                point2(center.x - rx * KAPPA90, center.y - ry),
                point2(center.x - rx, center.y - ry * KAPPA90),
            )
            .close_path()
    }

    pub fn transform(mut self, t: impl Into<Transform2D<f32>>) -> Self {
        let t: Transform2D<f32> = t.into();

        if let Some(xform) = &mut self.xform {
            *xform = t.then(xform);
        } else {
            self.xform = Some(t);
        }

        self
    }

    pub fn reset_transform(mut self) -> Self {
        self.xform = None;
        self
    }

    pub fn translate(mut self, v: impl Into<Vector2D<f32>>) -> Self {
        let v: Vector2D<f32> = v.into();

        if let Some(xform) = &mut self.xform {
            *xform = xform.pre_translate(v);
        } else {
            self.xform = Some(Transform2D::translation(v.x, v.y));
        }

        self
    }

    pub fn rotate(mut self, angle: impl Into<Angle<f32>>) -> Self {
        let angle: Angle<f32> = angle.into();

        if let Some(xform) = &mut self.xform {
            *xform = xform.pre_rotate(angle);
        } else {
            self.xform = Some(Transform2D::rotation(angle));
        }

        self
    }

    pub fn scale(mut self, x: f32, y: f32) -> Self {
        if let Some(xform) = &mut self.xform {
            *xform = xform.pre_scale(x, y);
        } else {
            self.xform = Some(Transform2D::scale(x, y));
        }

        self
    }

    pub fn skew_x(mut self, angle: impl Into<Angle<f32>>) -> Self {
        let angle: Angle<f32> = angle.into();

        if let Some(xform) = &mut self.xform {
            *xform = crate::math::transform_skew_x(angle).then(&xform)
        } else {
            self.xform = Some(crate::math::transform_skew_x(angle));
        }

        self
    }

    pub fn skew_y(mut self, angle: impl Into<Angle<f32>>) -> Self {
        let angle: Angle<f32> = angle.into();

        if let Some(xform) = &mut self.xform {
            *xform = crate::math::transform_skew_y(angle).then(&xform)
        } else {
            self.xform = Some(crate::math::transform_skew_y(angle));
        }

        self
    }

    pub fn current_transform(&self) -> Transform2D<f32> {
        self.xform.unwrap_or(Transform2D::default())
    }

    /// Build the mesh, caching the results.
    ///
    /// Paths which have the same commands and [`MeshOpts`] will be cached (including
    /// paths constructed across different frames), avoiding the need to re-flatten
    /// and re-tessellate the paths, as well as allowing for more efficient memory
    /// usage and fewer copy operations by re-using vertex buffers.
    ///
    /// This can be useful if you are drawing a bunch of elements that have geometry
    /// which mostly stays static across frames and/or you have many elements that
    /// share the same geometry but at different offsets.
    ///
    /// NOTE, paths that have the same geometry but which start at different offsets
    /// are *NOT* considered equal. To make proper use  of this method, build the
    /// paths with no offset and then apply the appropriate offset afterwards when
    /// painting the mesh.
    ///
    /// ALSO NOTE that this caching has some overhead. If you know that the geometry
    /// is not normally static, then consider using [`MeshBuilder::build_uncached`]
    /// or [`MeshBuilder::build_uncached_reusing_alloc`] instead.
    pub fn build(self, opts: MeshOpts) -> CachedMeshID {
        self.r
            .mesh_cache_key
            .set_mesh_opts(&opts, self.r.antialiasing_enabled);

        self.r
            .mesh_cache
            .build_mesh_cached(&mut self.r.tessellator, &self.r.mesh_cache_key)
    }

    /// Build the mesh.
    ///
    /// Unlike [`MeshBuilder::build`], this method always flattens and tessellates paths
    /// in favor of avoiding caching overhead.
    ///
    /// This is useful for geometry which does not usually stay static across frames.
    pub fn build_uncached(self, opts: MeshOpts) -> UncachedMeshID {
        self.r
            .mesh_cache_key
            .set_mesh_opts(&opts, self.r.antialiasing_enabled);

        self.r.mesh_cache.build_mesh_uncached(
            &mut self.r.tessellator,
            &self.r.mesh_cache_key,
            &mut UncachedMeshID::default(),
        )
    }

    /// Build the mesh, re-using the allocations from the previous frame to improve
    /// performance.
    ///
    /// Unlike [`MeshBuilder::build`], this method always flattens and tessellates paths
    /// in favor of avoiding caching overhead.
    ///
    /// This is useful for geometry which does not usually stay static across frames, and
    /// which contains large amounts of vertices (i.e. a spectrum analyzer).
    ///
    /// If `mesh_id == UncachedMeshID::default()` (a dangling ID), then a new mesh
    /// slot will be allocated and `mesh_id` will be replaced with the new valid
    /// ID.
    pub fn build_uncached_reusing_alloc(self, opts: MeshOpts, mesh_id: &mut UncachedMeshID) {
        self.r
            .mesh_cache_key
            .set_mesh_opts(&opts, self.r.antialiasing_enabled);

        self.r.mesh_cache.build_mesh_uncached(
            &mut self.r.tessellator,
            &self.r.mesh_cache_key,
            mesh_id,
        );
    }
}

fn sign_of(x: f32) -> f32 {
    if x >= 0.0 {
        1.0
    } else {
        -1.0
    }
}
