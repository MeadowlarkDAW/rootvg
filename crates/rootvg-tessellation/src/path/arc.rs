// The following code was copied and modified from
// https://github.com/iced-rs/iced/blob/31d1d5fecbef50fa319cabd5d4194f1e4aaefa21/graphics/src/geometry/path/arc.rs
// Iced license (MIT): https://github.com/iced-rs/iced/blob/31d1d5fecbef50fa319cabd5d4194f1e4aaefa21/LICENSE

//! Build and draw curves.
use rootvg_core::math::{Angle, Point, Vector};

/// A segment of a differentiable curve.
#[derive(Debug, Clone, Copy)]
pub struct ArcPath {
    /// The center of the arc.
    pub center: Point,
    /// The radius of the arc.
    pub radius: f32,
    /// The start of the segment's angle, clockwise rotation from positive x-axis.
    pub start_angle: Angle,
    /// The end of the segment's angle, clockwise rotation from positive x-axis.
    pub end_angle: Angle,
}

/// An elliptical [`ArcPath`].
#[derive(Debug, Clone, Copy)]
pub struct EllipticalArcPath {
    /// The center of the arc.
    pub center: Point,
    /// The radii of the arc's ellipse. The horizontal and vertical half-dimensions of the ellipse will match the x and y values of the radii vector.
    pub radii: Vector,
    /// The clockwise rotation of the arc's ellipse.
    pub rotation: Angle,
    /// The start of the segment's angle, clockwise rotation from positive x-axis.
    pub start_angle: Angle,
    /// The end of the segment's angle, clockwise rotation from positive x-axis.
    pub end_angle: Angle,
}

impl From<ArcPath> for EllipticalArcPath {
    fn from(arc: ArcPath) -> Self {
        Self {
            center: arc.center,
            radii: Vector::new(arc.radius, arc.radius),
            rotation: Angle::default(),
            start_angle: arc.start_angle,
            end_angle: arc.end_angle,
        }
    }
}
