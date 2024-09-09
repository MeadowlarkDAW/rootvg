use euclid::default::{Point2D, Vector2D};

mod builder;
pub(crate) mod cache;
mod commands;
pub(crate) mod tessellator;

use self::builder::MeshBuilderInner;
use self::commands::{Command, CommandIterator, PackedCommandBuffer};
use self::tessellator::Tessellator;

pub use self::builder::MeshBuilder;
pub use self::cache::MeshID;

pub const DEFAULT_MITER_LIMIT: f32 = 10.0;

/// Length proportional to radius of a cubic bezier handle for 90deg arcs
const KAPPA90: f32 = 0.5522847493;
const ONE_MINUS_KAPPA90: f32 = 1.0 - KAPPA90;

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
