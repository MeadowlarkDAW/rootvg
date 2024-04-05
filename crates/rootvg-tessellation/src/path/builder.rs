// The following code was copied and modified from
// https://github.com/iced-rs/iced/blob/31d1d5fecbef50fa319cabd5d4194f1e4aaefa21/graphics/src/geometry/path/builder.rs
// Iced license (MIT): https://github.com/iced-rs/iced/blob/31d1d5fecbef50fa319cabd5d4194f1e4aaefa21/LICENSE

use super::{ArcPath, EllipticalArcPath, Path};

use rootvg_core::math::{Angle, Point, Size};

use lyon::geom;
use lyon::math;
use lyon::path::builder::{self, SvgPathBuilder};

/// A [`Path`] builder.
///
/// Once a [`Path`] is built, it can no longer be mutated.
pub struct PathBuilder {
    pub raw: builder::WithSvg<lyon::path::path::BuilderImpl>,
}

impl PathBuilder {
    /// Creates a new [`Builder`].
    pub fn new() -> Self {
        Self {
            raw: lyon::path::Path::builder().with_svg(),
        }
    }

    /// Moves the starting point of a new sub-path to the given `Point`.
    pub fn move_to(mut self, point: Point) -> Self {
        self.raw.move_to(math::Point::new(point.x, point.y));
        self
    }

    /// Connects the last point in the [`Path`] to the given `Point` with a
    /// straight line.
    pub fn line_to(mut self, point: Point) -> Self {
        self.raw.line_to(math::Point::new(point.x, point.y));
        self
    }

    /// Adds an [`Arc`] to the [`Path`] from `start_angle` to `end_angle` in
    /// a clockwise direction.
    pub fn arc(self, arc: ArcPath) -> Self {
        self.ellipse(arc.into())
    }

    /// Adds a circular arc to the [`Path`] with the given control points and
    /// radius.
    ///
    /// This essentially draws a straight line segment from the current
    /// position to `a`, but fits a circular arc of `radius` tangent to that
    /// segment and tangent to the line between `a` and `b`.
    ///
    /// With another `.line_to(b)`, the result will be a path connecting the
    /// starting point and `b` with straight line segments towards `a` and a
    /// circular arc smoothing out the corner at `a`.
    ///
    /// See [the HTML5 specification of `arcTo`](https://html.spec.whatwg.org/multipage/canvas.html#building-paths:dom-context-2d-arcto)
    /// for more details and examples.
    pub fn arc_to(mut self, a: Point, b: Point, radius: f32) -> Self {
        let start = self.raw.current_position();
        let mid = math::Point::new(a.x, a.y);
        let end = math::Point::new(b.x, b.y);

        if start == mid || mid == end || radius == 0.0 {
            let _ = self.raw.line_to(mid);
            return self;
        }

        let double_area =
            start.x * (mid.y - end.y) + mid.x * (end.y - start.y) + end.x * (start.y - mid.y);

        if double_area == 0.0 {
            let _ = self.raw.line_to(mid);
            return self;
        }

        let to_start = (start - mid).normalize();
        let to_end = (end - mid).normalize();

        let inner_angle = to_start.dot(to_end).acos();

        let origin_angle = inner_angle / 2.0;

        let origin_adjacent = radius / origin_angle.tan();

        let arc_start = mid + to_start * origin_adjacent;
        let arc_end = mid + to_end * origin_adjacent;

        let sweep = to_start.cross(to_end) < 0.0;

        let _ = self.raw.line_to(arc_start);

        self.raw.arc_to(
            math::Vector::new(radius, radius),
            math::Angle::radians(0.0),
            lyon::path::ArcFlags {
                large_arc: false,
                sweep,
            },
            arc_end,
        );

        self
    }

    /// Adds an ellipse to the [`Path`] using a clockwise direction.
    pub fn ellipse(mut self, arc: EllipticalArcPath) -> Self {
        let arc = geom::Arc {
            center: math::Point::new(arc.center.x, arc.center.y),
            radii: math::Vector::new(arc.radii.x, arc.radii.y),
            x_rotation: math::Angle::radians(arc.rotation.radians),
            start_angle: math::Angle::radians(arc.start_angle.radians),
            sweep_angle: math::Angle::radians((arc.end_angle - arc.start_angle).radians),
        };

        let _ = self.raw.move_to(arc.sample(0.0));

        arc.for_each_quadratic_bezier(&mut |curve| {
            let _ = self.raw.quadratic_bezier_to(curve.ctrl, curve.to);
        });

        self
    }

    /// Adds a cubic Bézier curve to the [`Path`] given its two control points
    /// and its end point.
    pub fn bezier_curve_to(mut self, control_a: Point, control_b: Point, to: Point) -> Self {
        let _ = self.raw.cubic_bezier_to(
            math::Point::new(control_a.x, control_a.y),
            math::Point::new(control_b.x, control_b.y),
            math::Point::new(to.x, to.y),
        );
        self
    }

    /// Adds a quadratic Bézier curve to the [`Path`] given its control point
    /// and its end point.
    pub fn quadratic_curve_to(mut self, control: Point, to: Point) -> Self {
        let _ = self.raw.quadratic_bezier_to(
            math::Point::new(control.x, control.y),
            math::Point::new(to.x, to.y),
        );
        self
    }

    /// Adds a rectangle to the [`Path`] given its top-left corner coordinate
    /// and its `Size`.
    pub fn rectangle(self, top_left: Point, size: Size) -> Self {
        self.move_to(top_left)
            .line_to(Point::new(top_left.x + size.width, top_left.y))
            .line_to(Point::new(
                top_left.x + size.width,
                top_left.y + size.height,
            ))
            .line_to(Point::new(top_left.x, top_left.y + size.height))
            .close()
    }

    /// Adds a circle to the [`Path`] given its center coordinate and its
    /// radius.
    pub fn circle(self, center: Point, radius: f32) -> Self {
        self.arc(ArcPath {
            center,
            radius,
            start_angle: Angle { radians: 0.0 },
            end_angle: Angle {
                radians: 2.0 * std::f32::consts::PI,
            },
        })
    }

    /// Closes the current sub-path in the [`Path`] with a straight line to
    /// the starting point.
    pub fn close(mut self) -> Self {
        self.raw.close();
        self
    }

    /// Builds the [`Path`] of this [`PathBuilder`].
    pub fn build(self) -> Path {
        Path {
            raw: self.raw.build(),
        }
    }
}

impl Default for PathBuilder {
    fn default() -> Self {
        Self::new()
    }
}
