use std::ops::{Div, Mul};

use euclid::UnknownUnit;

pub type ZIndex = u16;

/// Units in physical pixels.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Physical;

/// A point in units of logical points.
///
/// Alias for ```euclid::default::Point2D<f32>```.
pub type Point = euclid::default::Point2D<f32>;

/// A point in units of logical points.
///
/// Alias for ```euclid::default::Point2D<i32>```.
pub type PointI32 = euclid::default::Point2D<i32>;

/// A point in units of logical points.
///
/// Alias for ```euclid::default::Point2D<f64>```.
pub type PointF64 = euclid::default::Point2D<f64>;

/// A vector in units of logical points.
///
/// Alias for ```euclid::default::Vector2D<f32>```.
pub type Vector = euclid::default::Vector2D<f32>;

/// A vector in units of logical points.
///
/// Alias for ```euclid::default::Vector2D<f64>```.
pub type VectorF64 = euclid::default::Vector2D<f64>;

/// A size in units of logical points.
///
/// Alias for ```euclid::default::Size2D<f32>```.
pub type Size = euclid::default::Size2D<f32>;

/// A size in units of logical points.
///
/// Alias for ```euclid::default::Size2D<i32>```.
pub type SizeI32 = euclid::default::Size2D<i32>;

/// A size in units of logical points.
///
/// Alias for ```euclid::default::Size2D<f64>```.
pub type SizeF64 = euclid::default::Size2D<f64>;

/// Alias for ```euclid::default::Box2D<f32>```
pub type Box2D = euclid::default::Box2D<f32>;

/// Alias for ```euclid::default::Box2D<f64>```
pub type Box2DF64 = euclid::default::Box2D<f64>;

/// Alias for ```euclid::default::Transform2D<f32>```
pub type Transform = euclid::default::Transform2D<f32>;

/// Alias for ```euclid::default::Transform2D<f64>```
pub type TransformF64 = euclid::default::Transform2D<f64>;

/// Alias for ```euclid::default::Rotation2D<f32>```
pub type Rotation = euclid::default::Rotation2D<f32>;

/// Alias for ```euclid::default::Rotation2D<f64>```
pub type RotationF64 = euclid::default::Rotation2D<f64>;

/// Alias for ```euclid::default::Translation2D<f32>```
pub type Translation = euclid::Translation2D<f32, euclid::UnknownUnit, euclid::UnknownUnit>;

/// Alias for ```euclid::default::Translation2D<f64>```
pub type TranslationF64 = euclid::Translation2D<f64, euclid::UnknownUnit, euclid::UnknownUnit>;

/// Alias for ```euclid::default::Scale<f32>```
pub type Scale = euclid::default::Scale<f32>;

/// Alias for ```euclid::default::Scale<f64>```
pub type ScaleF64 = euclid::default::Scale<f64>;

/// A rectangle in units of logical points.
///
/// Alias for ```euclid::default::Rect<f32>```
pub type Rect = euclid::default::Rect<f32>;

/// A rectangle in units of logical points.
///
/// Alias for ```euclid::default::Rect<i32>```
pub type RectI32 = euclid::default::Rect<i32>;

/// A rectangle in units of logical points.
///
/// Alias for ```euclid::default::Rect<f64>```
pub type RectF64 = euclid::default::Rect<f64>;

/// An angle in radians (f32).
///
/// Alias for ```euclid::Angle<f32>```
pub type Angle = euclid::Angle<f32>;

/// An angle in radians (f64).
///
/// Alias for ```euclid::Angle<f64>```
pub type AngleF64 = euclid::Angle<f64>;

/// A group of 2D side offsets, which correspond to top/right/bottom/left for borders,
/// padding,and margins in CSS, optionally tagged with a unit.
///
/// Alias for ```euclid::SideOffsets2D<f32, UnknownUnit>```
pub type SideOffsets = euclid::SideOffsets2D<f32, UnknownUnit>;

/// A group of 2D side offsets, which correspond to top/right/bottom/left for borders,
/// padding,and margins in CSS, optionally tagged with a unit.
///
/// Alias for ```euclid::SideOffsets2D<f64, UnknownUnit>```
pub type SideOffsetsF64 = euclid::SideOffsets2D<f64, UnknownUnit>;

/*
/// A point in units of logical points.
pub type LogicalPoint = euclid::Point2D<f32, Logical>;
*/
/// A point in units of physical pixels.
pub type PhysicalPoint = euclid::Point2D<f32, Physical>;
/// A point in units of physical pixels.
pub type PhysicalPointU32 = euclid::Point2D<u32, Physical>;
/// A point in units of physical pixels.
pub type PhysicalPointI32 = euclid::Point2D<i32, Physical>;

/*
/// A size in units of logical points.
pub type LogicalSize = euclid::Size2D<f32, Logical>;
*/
/// A size in units of physical pixels.
pub type PhysicalSize = euclid::Size2D<f32, Physical>;
/// A size in units of physical pixels.
pub type PhysicalSizeU32 = euclid::Size2D<u32, Physical>;
/// A size in units of physical pixels.
pub type PhysicalSizeI32 = euclid::Size2D<i32, Physical>;

/*
/// A rectangle in units of logical points.
pub type LogicalRect = euclid::Rect<f32, Logical>;
*/
/// A rectagngle in units of physical pixels.
pub type PhysicalRect = euclid::Rect<f32, Physical>;
/// A rectagngle in units of physical pixels.
pub type PhysicalRectU32 = euclid::Rect<u32, Physical>;
/// A rectagngle in units of physical pixels.
pub type PhysicalRectI32 = euclid::Rect<i32, Physical>;

/// Convert a point from logical points to physical pixels.
#[inline]
pub fn to_physical_point(point: Point, scale_factor: ScaleFactor) -> PhysicalPoint {
    PhysicalPoint::new(point.x * scale_factor.0, point.y * scale_factor.0)
}
/// Convert a point from logical points to physical pixels.
#[inline]
pub fn to_logical_point(point: PhysicalPoint, scale_factor: ScaleFactor) -> Point {
    Point::new(point.x / scale_factor.0, point.y / scale_factor.0)
}
/// Convert a point from logical points to physical pixels.
#[inline]
pub fn to_logical_point_u32(point: PhysicalPointU32, scale_factor: ScaleFactor) -> Point {
    Point::new(
        point.x as f32 / scale_factor.0,
        point.y as f32 / scale_factor.0,
    )
}
/// Convert a point from logical points to physical pixels.
#[inline]
pub fn to_logical_point_i32(point: PhysicalPointI32, scale_factor: ScaleFactor) -> Point {
    Point::new(
        point.x as f32 / scale_factor.0,
        point.y as f32 / scale_factor.0,
    )
}
/// Convert a point from logical points to physical pixels using the reciprocal of the scale factor.
#[inline]
pub fn to_logical_point_from_recip(point: PhysicalPoint, scale_factor_recip: f32) -> Point {
    Point::new(point.x * scale_factor_recip, point.y * scale_factor_recip)
}
/// Convert a point from logical points to physical pixels using the reciprocal of the scale factor.
#[inline]
pub fn to_logical_point_from_recip_u32(point: PhysicalPointU32, scale_factor_recip: f32) -> Point {
    Point::new(
        point.x as f32 * scale_factor_recip,
        point.y as f32 * scale_factor_recip,
    )
}
/// Convert a point from logical points to physical pixels using the reciprocal of the scale factor.
#[inline]
pub fn to_logical_point_from_recip_i32(point: PhysicalPointI32, scale_factor_recip: f32) -> Point {
    Point::new(
        point.x as f32 * scale_factor_recip,
        point.y as f32 * scale_factor_recip,
    )
}

/// Convert a size from logical points to physical pixels.
#[inline]
pub fn to_physical_size(size: Size, scale_factor: ScaleFactor) -> PhysicalSize {
    PhysicalSize::new(size.width * scale_factor.0, size.height * scale_factor.0)
}
/// Convert a size from logical points to physical pixels.
#[inline]
pub fn to_logical_size(size: PhysicalSize, scale_factor: ScaleFactor) -> Size {
    Size::new(size.width / scale_factor.0, size.height / scale_factor.0)
}
/// Convert a size from logical points to physical pixels.
#[inline]
pub fn to_logical_size_u32(size: PhysicalSizeU32, scale_factor: ScaleFactor) -> Size {
    Size::new(
        size.width as f32 / scale_factor.0,
        size.height as f32 / scale_factor.0,
    )
}
/// Convert a size from logical points to physical pixels.
#[inline]
pub fn to_logical_size_i32(size: PhysicalSizeI32, scale_factor: ScaleFactor) -> Size {
    Size::new(
        size.width as f32 / scale_factor.0,
        size.height as f32 / scale_factor.0,
    )
}
/// Convert a size from logical points to physical pixels using the reciprocal of the scale factor.
#[inline]
pub fn to_logical_size_from_recip(size: PhysicalSize, scale_factor_recip: f32) -> Size {
    Size::new(
        size.width * scale_factor_recip,
        size.height * scale_factor_recip,
    )
}
/// Convert a size from logical points to physical pixels using the reciprocal of the scale factor.
#[inline]
pub fn to_logical_size_from_recip_u32(size: PhysicalSizeU32, scale_factor_recip: f32) -> Size {
    Size::new(
        size.width as f32 * scale_factor_recip,
        size.height as f32 * scale_factor_recip,
    )
}
/// Convert a size from logical points to physical pixels using the reciprocal of the scale factor.
#[inline]
pub fn to_logical_size_from_recip_i32(size: PhysicalSizeI32, scale_factor_recip: f32) -> Size {
    Size::new(
        size.width as f32 * scale_factor_recip,
        size.height as f32 * scale_factor_recip,
    )
}

/// Convert a rectangle from logical points to physical pixels.
#[inline]
pub fn to_physical_rect(rect: Rect, scale_factor: ScaleFactor) -> PhysicalRect {
    PhysicalRect::new(
        to_physical_point(rect.origin, scale_factor),
        to_physical_size(rect.size, scale_factor),
    )
}
/// Convert a rectangle from logical points to physical pixels.
#[inline]
pub fn to_logical_rect(rect: PhysicalRect, scale_factor: ScaleFactor) -> Rect {
    Rect::new(
        to_logical_point(rect.origin, scale_factor),
        to_logical_size(rect.size, scale_factor),
    )
}
/// Convert a rectangle from logical points to physical pixels.
#[inline]
pub fn to_logical_rect_u32(rect: PhysicalRectU32, scale_factor: ScaleFactor) -> Rect {
    Rect::new(
        to_logical_point_u32(rect.origin, scale_factor),
        to_logical_size_u32(rect.size, scale_factor),
    )
}
/// Convert a rectangle from logical points to physical pixels.
#[inline]
pub fn to_logical_rect_i32(rect: PhysicalRectI32, scale_factor: ScaleFactor) -> Rect {
    Rect::new(
        to_logical_point_i32(rect.origin, scale_factor),
        to_logical_size_i32(rect.size, scale_factor),
    )
}
/// Convert a rectangle from logical points to physical pixels using the reciprocal of the scale factor.
#[inline]
pub fn to_logical_rect_from_recip(rect: PhysicalRect, scale_factor_recip: f32) -> Rect {
    Rect::new(
        to_logical_point_from_recip(rect.origin, scale_factor_recip),
        to_logical_size_from_recip(rect.size, scale_factor_recip),
    )
}
/// Convert a rectangle from logical points to physical pixels using the reciprocal of the scale factor.
#[inline]
pub fn to_logical_rect_from_recip_u32(rect: PhysicalRectU32, scale_factor_recip: f32) -> Rect {
    Rect::new(
        to_logical_point_from_recip_u32(rect.origin, scale_factor_recip),
        to_logical_size_from_recip_u32(rect.size, scale_factor_recip),
    )
}
/// Convert a rectangle from logical points to physical pixels using the reciprocal of the scale factor.
#[inline]
pub fn to_logical_rect_from_recip_i32(rect: PhysicalRectI32, scale_factor_recip: f32) -> Rect {
    Rect::new(
        to_logical_point_from_recip_i32(rect.origin, scale_factor_recip),
        to_logical_size_from_recip_i32(rect.size, scale_factor_recip),
    )
}

/// Shorthand for `Vector::new(x, y)`.
#[inline]
pub const fn vector(x: f32, y: f32) -> Vector {
    Vector::new(x, y)
}

/// Shorthand for `Point::new(x, y)`.
#[inline]
pub const fn point(x: f32, y: f32) -> Point {
    Point::new(x, y)
}

/// Shorthand for `Size::new(x, y)`.
#[inline]
pub const fn size(w: f32, h: f32) -> Size {
    Size::new(w, h)
}

/// Shorthand for `Angle { radians: value }`.
#[inline]
pub const fn radians(radians: f32) -> Angle {
    Angle { radians }
}

/// Shorthand for `Angle { radians: value * PI / 180.0 }`.
#[inline]
pub fn degrees(degrees: f32) -> Angle {
    Angle {
        radians: degrees * (std::f32::consts::PI / 180.0),
    }
}

/// Shorthand for `Rect::new(Point::new(x, y), Size::new(width, height))`.
#[inline]
pub const fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::new(Point::new(x, y), Size::new(width, height))
}

/// A scaling factor in points per pixel.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ScaleFactor(pub f32);

impl ScaleFactor {
    pub fn new(scale_factor: f32) -> Self {
        Self(scale_factor)
    }

    pub fn recip(&self) -> f32 {
        self.0.recip()
    }
}

impl Default for ScaleFactor {
    fn default() -> Self {
        Self(1.0)
    }
}

impl From<f32> for ScaleFactor {
    fn from(s: f32) -> Self {
        Self(s)
    }
}

impl From<f64> for ScaleFactor {
    fn from(s: f64) -> Self {
        Self(s as f32)
    }
}

impl From<ScaleFactor> for f32 {
    fn from(s: ScaleFactor) -> Self {
        s.0
    }
}

impl From<ScaleFactor> for f64 {
    fn from(s: ScaleFactor) -> Self {
        s.0 as f64
    }
}

impl Mul<ScaleFactor> for f32 {
    type Output = f32;
    fn mul(self, rhs: ScaleFactor) -> Self::Output {
        self * rhs.0
    }
}

impl Div<ScaleFactor> for f32 {
    type Output = f32;
    fn div(self, rhs: ScaleFactor) -> Self::Output {
        self / rhs.0
    }
}

/// Returns a scaling vector that can be used to convert screen coordinates
/// to clip coordinates in shaders.
pub fn screen_to_clip_scale(screen_size: PhysicalSizeI32, scale_factor: ScaleFactor) -> [f32; 2] {
    [
        2.0 * scale_factor * (screen_size.width as f32).recip(),
        2.0 * scale_factor * (screen_size.height as f32).recip(),
    ]
}
