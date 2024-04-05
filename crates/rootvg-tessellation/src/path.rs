// The following code was copied and modified from
// https://github.com/iced-rs/iced/blob/31d1d5fecbef50fa319cabd5d4194f1e4aaefa21/graphics/src/geometry/path.rs
// Iced license (MIT): https://github.com/iced-rs/iced/blob/31d1d5fecbef50fa319cabd5d4194f1e4aaefa21/LICENSE

mod arc;
mod builder;

#[doc(no_inline)]
pub use arc::{ArcPath, EllipticalArcPath};
pub use builder::PathBuilder;

pub use lyon::path as lyon_path;

use rootvg_core::math::{Point, Size};

/// An immutable set of points that may or may not be connected.
///
/// A single [`Path`] can represent different kinds of 2D shapes!
#[derive(Debug, Clone)]
pub struct Path {
    pub raw: lyon::path::Path,
}

impl Path {
    pub fn builder() -> PathBuilder {
        PathBuilder::new()
    }

    /// Creates a new [`Path`] representing a line segment given its starting
    /// and end points.
    pub fn line(from: Point, to: Point) -> Self {
        PathBuilder::new().move_to(from).line_to(to).build()
    }

    /// Creates a new [`Path`] representing a rectangle given its top-left
    /// corner coordinate and its `Size`.
    pub fn rectangle(top_left: Point, size: Size) -> Self {
        PathBuilder::new().rectangle(top_left, size).build()
    }

    /// Creates a new [`Path`] representing a circle given its center
    /// coordinate and its radius.
    pub fn circle(center: Point, radius: f32) -> Self {
        PathBuilder::new().circle(center, radius).build()
    }

    /// Returns the current [`Path`] with the given transform applied to it.
    pub fn transform(&self, transform: &lyon::path::math::Transform) -> Path {
        Path {
            raw: self.raw.clone().transformed(transform),
        }
    }
}
