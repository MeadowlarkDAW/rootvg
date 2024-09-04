use euclid::default::{Point2D, Size2D, Transform2D};
use euclid::Angle;

pub use euclid;

pub fn transform_skew_x(angle: Angle<f32>) -> Transform2D<f32> {
    Transform2D::new(1.0, 0.0, angle.radians.tan(), 1.0, 0.0, 0.0)
}

pub fn transform_skew_y(angle: Angle<f32>) -> Transform2D<f32> {
    Transform2D::new(1.0, angle.radians.tan(), 0.0, 1.0, 0.0, 0.0)
}
