// The following code was copied and modified from
// https://github.com/iced-rs/iced/blob/31d1d5fecbef50fa319cabd5d4194f1e4aaefa21/wgpu/src/geometry.rs
// Iced license (MIT): https://github.com/iced-rs/iced/blob/31d1d5fecbef50fa319cabd5d4194f1e4aaefa21/LICENSE

use lyon::tessellation;
use std::borrow::Cow;
use std::rc::Rc;

use rootvg_core::color::PackedSrgb;
use rootvg_core::math::{Angle, Point, Size, Vector};
use rootvg_mesh::{
    Indexed, MeshPrimitive, MeshUniforms, SolidMesh, SolidMeshPrimitive, SolidVertex2D,
};

#[cfg(feature = "gradient")]
use rootvg_core::gradient::PackedGradient;
#[cfg(feature = "gradient")]
use rootvg_mesh::{GradientMesh, GradientMeshPrimitive, GradientVertex2D};

use crate::fill::{Fill, FillRule, FillStyle};
use crate::path::{Path, PathBuilder};
use crate::stroke::{LineCap, LineDash, LineJoin, Stroke};

/// A frame for drawing some meshes with a solid fill.
#[allow(missing_debug_implementations)]
pub struct Tessellator {
    buffers: BufferStack,
    primitives: Vec<MeshPrimitive>,
    transforms: Transforms,
    fill_tessellator: tessellation::FillTessellator,
    stroke_tessellator: tessellation::StrokeTessellator,
}

enum Buffer {
    Solid(tessellation::VertexBuffers<SolidVertex2D, u32>),
    #[cfg(feature = "gradient")]
    Gradient(tessellation::VertexBuffers<GradientVertex2D, u32>),
}

struct BufferStack {
    stack: Vec<Buffer>,
}

impl BufferStack {
    fn new() -> Self {
        Self { stack: Vec::new() }
    }

    fn get_mut(&mut self, style: &FillStyle) -> &mut Buffer {
        match style {
            FillStyle::Solid(_) => match self.stack.last() {
                Some(Buffer::Solid(_)) => {}
                _ => {
                    self.stack
                        .push(Buffer::Solid(tessellation::VertexBuffers::new()));
                }
            },
            #[cfg(feature = "gradient")]
            FillStyle::Gradient(_) => match self.stack.last() {
                Some(Buffer::Gradient(_)) => {}
                _ => {
                    self.stack
                        .push(Buffer::Gradient(tessellation::VertexBuffers::new()));
                }
            },
        }

        self.stack.last_mut().unwrap()
    }

    fn get_fill<'a>(
        &'a mut self,
        style: &FillStyle,
    ) -> Box<dyn tessellation::FillGeometryBuilder + 'a> {
        match (style, self.get_mut(style)) {
            (FillStyle::Solid(color), Buffer::Solid(buffer)) => Box::new(
                tessellation::BuffersBuilder::new(buffer, TriangleVertex2DBuilder(*color)),
            ),
            #[cfg(feature = "gradient")]
            (FillStyle::Gradient(gradient), Buffer::Gradient(buffer)) => {
                Box::new(tessellation::BuffersBuilder::new(
                    buffer,
                    GradientVertex2DBuilder {
                        gradient: *gradient,
                    },
                ))
            }
            #[cfg(feature = "gradient")]
            _ => unreachable!(),
        }
    }

    fn get_stroke<'a>(
        &'a mut self,
        style: &FillStyle,
    ) -> Box<dyn tessellation::StrokeGeometryBuilder + 'a> {
        match (style, self.get_mut(style)) {
            (FillStyle::Solid(color), Buffer::Solid(buffer)) => Box::new(
                tessellation::BuffersBuilder::new(buffer, TriangleVertex2DBuilder(*color)),
            ),
            #[cfg(feature = "gradient")]
            (FillStyle::Gradient(gradient), Buffer::Gradient(buffer)) => {
                Box::new(tessellation::BuffersBuilder::new(
                    buffer,
                    GradientVertex2DBuilder {
                        gradient: *gradient,
                    },
                ))
            }
            #[cfg(feature = "gradient")]
            _ => unreachable!(),
        }
    }
}

#[derive(Debug)]
struct Transforms {
    previous: Vec<Transform>,
    current: Transform,
}

#[derive(Debug, Clone, Copy)]
struct Transform(lyon::math::Transform);

impl Transform {
    fn is_identity(&self) -> bool {
        self.0 == lyon::math::Transform::identity()
    }

    /*
    fn is_scale_translation(&self) -> bool {
        self.0.m12.abs() < 2.0 * f32::EPSILON && self.0.m21.abs() < 2.0 * f32::EPSILON
    }

    fn scale(&self) -> (f32, f32) {
        (self.0.m11, self.0.m22)
    }
    */

    #[cfg(feature = "gradient")]
    fn transform_point(&self, point: Point) -> Point {
        let transformed = self
            .0
            .transform_point(lyon::geom::euclid::Point2D::new(point.x, point.y));

        Point::new(transformed.x, transformed.y)
    }

    fn transform_style(&self, style: FillStyle) -> FillStyle {
        match style {
            FillStyle::Solid(color) => FillStyle::Solid(color),
            #[cfg(feature = "gradient")]
            FillStyle::Gradient(gradient) => FillStyle::Gradient(self.transform_gradient(gradient)),
        }
    }

    #[cfg(feature = "gradient")]
    fn transform_gradient(&self, mut gradient: PackedGradient) -> PackedGradient {
        let start = self.transform_point(Point::new(gradient.direction[0], gradient.direction[1]));
        let end = self.transform_point(Point::new(gradient.direction[2], gradient.direction[3]));

        gradient.direction[0] = start.x;
        gradient.direction[1] = start.y;
        gradient.direction[2] = end.x;
        gradient.direction[3] = end.y;

        gradient
    }
}

impl Default for Tessellator {
    fn default() -> Self {
        Self {
            buffers: BufferStack::new(),
            primitives: Vec::new(),
            transforms: Transforms {
                previous: Vec::new(),
                current: Transform(lyon::math::Transform::identity()),
            },
            fill_tessellator: tessellation::FillTessellator::new(),
            stroke_tessellator: tessellation::StrokeTessellator::new(),
        }
    }
}

impl Tessellator {
    /// Creates a new empty [`Tessellator`] with the given dimensions.
    ///
    /// The default coordinate system of a [`Tessellator`] has its origin at the
    /// top-left corner of its bounds.
    pub fn new() -> Self {
        Self::default()
    }

    /// Draws the given [`Path`] on the [`Tessellator`] by filling it with the
    /// provided style.
    pub fn fill(mut self, path: &Path, fill: impl Into<Fill>) -> Self {
        {
            let Fill { style, rule } = fill.into();

            let mut buffer = self
                .buffers
                .get_fill(&self.transforms.current.transform_style(style));

            let options = tessellation::FillOptions::default().with_fill_rule(into_fill_rule(rule));

            if self.transforms.current.is_identity() {
                self.fill_tessellator
                    .tessellate_path(&path.raw, &options, buffer.as_mut())
            } else {
                let path = path.transform(&self.transforms.current.0);

                self.fill_tessellator
                    .tessellate_path(&path.raw, &options, buffer.as_mut())
            }
            .expect("Tessellate path.");
        }

        self
    }

    /// Draws an axis-aligned rectangle given its top-left corner coordinate and
    /// its `Size` on the [`Tessellator`] by filling it with the provided style.
    pub fn fill_rectangle(mut self, top_left: Point, size: Size, fill: impl Into<Fill>) -> Self {
        {
            let Fill { style, rule } = fill.into();

            let mut buffer = self
                .buffers
                .get_fill(&self.transforms.current.transform_style(style));

            let top_left = self
                .transforms
                .current
                .0
                .transform_point(lyon::math::Point::new(top_left.x, top_left.y));

            let size = self
                .transforms
                .current
                .0
                .transform_vector(lyon::math::Vector::new(size.width, size.height));

            let options = tessellation::FillOptions::default().with_fill_rule(into_fill_rule(rule));

            self.fill_tessellator
                .tessellate_rectangle(
                    &lyon::math::Box2D::new(top_left, top_left + size),
                    &options,
                    buffer.as_mut(),
                )
                .expect("Fill rectangle");
        }

        self
    }

    /// Draws the stroke of the given [`Path`] on the [`Tessellator`] with the
    /// provided style.
    pub fn stroke<'a>(mut self, path: &Path, stroke: impl Into<Stroke<'a>>) -> Self {
        {
            let stroke: Stroke = stroke.into();

            let mut buffer = self
                .buffers
                .get_stroke(&self.transforms.current.transform_style(stroke.style));

            let mut options = tessellation::StrokeOptions::default();
            options.line_width = stroke.width;
            options.start_cap = into_line_cap(stroke.line_cap);
            options.end_cap = into_line_cap(stroke.line_cap);
            options.line_join = into_line_join(stroke.line_join);

            let path = if stroke.line_dash.segments.is_empty() {
                Cow::Borrowed(path)
            } else {
                Cow::Owned(dashed(path, stroke.line_dash))
            };

            if self.transforms.current.is_identity() {
                self.stroke_tessellator
                    .tessellate_path(&path.raw, &options, buffer.as_mut())
            } else {
                let path = path.transform(&self.transforms.current.0);

                self.stroke_tessellator
                    .tessellate_path(&path.raw, &options, buffer.as_mut())
            }
            .expect("Stroke path");
        }

        self
    }

    /*
    /// Stores the current transform of the [`Tessellator`] and executes the given
    /// drawing operations, restoring the transform afterwards.
    ///
    /// This method is useful to compose transforms and perform drawing
    /// operations in different coordinate systems.
    pub fn with_save<R>(&mut self, f: impl FnOnce(&mut Tessellator) -> R) -> R {
        self.transforms.previous.push(self.transforms.current);

        let result = f(self);

        self.transforms.current = self.transforms.previous.pop().unwrap();

        result
    }
    */

    /// Pushes the current transform in the transform stack.
    pub fn push_transform(mut self) -> Self {
        self.transforms.previous.push(self.transforms.current);
        self
    }

    /// Pops a transform from the transform stack and sets it as the current transform.
    pub fn pop_transform(mut self) -> Self {
        self.transforms.current = self.transforms.previous.pop().unwrap();
        self
    }

    /// Applies a translation to the current transform of the [`Tessellator`].
    pub fn translate(mut self, translation: Vector) -> Self {
        self.transforms.current.0 = self
            .transforms
            .current
            .0
            .pre_translate(lyon::math::Vector::new(translation.x, translation.y));

        self
    }

    /// Applies a rotation in radians to the current transform of the [`Tessellator`].
    pub fn rotate(mut self, angle: Angle) -> Self {
        self.transforms.current.0 = self.transforms.current.0.pre_rotate(angle);

        self
    }

    /// Applies a uniform scaling to the current transform of the [`Tessellator`].
    pub fn scale(self, scale: impl Into<f32>) -> Self {
        let scale = scale.into();

        self.scale_nonuniform(Vector::new(scale, scale))
    }

    /// Applies a non-uniform scaling to the current transform of the [`Tessellator`].
    pub fn scale_nonuniform(mut self, scale: impl Into<Vector>) -> Self {
        let scale: Vector = scale.into();

        self.transforms.current.0 = self.transforms.current.0.pre_scale(scale.x, scale.y);

        self
    }

    pub fn into_primitive(mut self) -> Option<MeshPrimitive> {
        let Some(buffer) = self.buffers.stack.drain(..).next() else {
            return None;
        };

        match buffer {
            Buffer::Solid(buffer) => {
                if !buffer.indices.is_empty() {
                    return Some(MeshPrimitive::Solid(SolidMeshPrimitive {
                        mesh: Rc::new(SolidMesh {
                            buffers: Indexed {
                                vertices: buffer.vertices,
                                indices: buffer.indices,
                            },
                        }),
                        uniform: MeshUniforms::default(),
                    }));
                }
            }
            #[cfg(feature = "gradient")]
            Buffer::Gradient(buffer) => {
                if !buffer.indices.is_empty() {
                    return Some(MeshPrimitive::Gradient(GradientMeshPrimitive {
                        mesh: Rc::new(GradientMesh {
                            buffers: Indexed {
                                vertices: buffer.vertices,
                                indices: buffer.indices,
                            },
                        }),
                        uniform: MeshUniforms::default(),
                    }));
                }
            }
        }

        None
    }

    pub fn into_primitive_batch(mut self) -> Vec<MeshPrimitive> {
        for buffer in self.buffers.stack {
            match buffer {
                Buffer::Solid(buffer) => {
                    if !buffer.indices.is_empty() {
                        self.primitives
                            .push(MeshPrimitive::Solid(SolidMeshPrimitive {
                                mesh: Rc::new(SolidMesh {
                                    buffers: Indexed {
                                        vertices: buffer.vertices,
                                        indices: buffer.indices,
                                    },
                                }),
                                uniform: MeshUniforms::default(),
                            }));
                    }
                }
                #[cfg(feature = "gradient")]
                Buffer::Gradient(buffer) => {
                    if !buffer.indices.is_empty() {
                        self.primitives
                            .push(MeshPrimitive::Gradient(GradientMeshPrimitive {
                                mesh: Rc::new(GradientMesh {
                                    buffers: Indexed {
                                        vertices: buffer.vertices,
                                        indices: buffer.indices,
                                    },
                                }),
                                uniform: MeshUniforms::default(),
                            }));
                    }
                }
            }
        }

        self.primitives
    }
}

struct TriangleVertex2DBuilder(PackedSrgb);

impl tessellation::FillVertexConstructor<SolidVertex2D> for TriangleVertex2DBuilder {
    fn new_vertex(&mut self, vertex: tessellation::FillVertex<'_>) -> SolidVertex2D {
        let position = vertex.position();

        SolidVertex2D {
            position: [position.x, position.y],
            color: self.0,
        }
    }
}

impl tessellation::StrokeVertexConstructor<SolidVertex2D> for TriangleVertex2DBuilder {
    fn new_vertex(&mut self, vertex: tessellation::StrokeVertex<'_, '_>) -> SolidVertex2D {
        let position = vertex.position();

        SolidVertex2D {
            position: [position.x, position.y],
            color: self.0,
        }
    }
}

#[cfg(feature = "gradient")]
struct GradientVertex2DBuilder {
    gradient: PackedGradient,
}

#[cfg(feature = "gradient")]
impl tessellation::FillVertexConstructor<GradientVertex2D> for GradientVertex2DBuilder {
    fn new_vertex(&mut self, vertex: tessellation::FillVertex<'_>) -> GradientVertex2D {
        let position = vertex.position();

        GradientVertex2D {
            position: [position.x, position.y],
            gradient: self.gradient,
        }
    }
}

#[cfg(feature = "gradient")]
impl tessellation::StrokeVertexConstructor<GradientVertex2D> for GradientVertex2DBuilder {
    fn new_vertex(&mut self, vertex: tessellation::StrokeVertex<'_, '_>) -> GradientVertex2D {
        let position = vertex.position();

        GradientVertex2D {
            position: [position.x, position.y],
            gradient: self.gradient,
        }
    }
}

fn into_line_join(line_join: LineJoin) -> lyon::tessellation::LineJoin {
    match line_join {
        LineJoin::Miter => lyon::tessellation::LineJoin::Miter,
        LineJoin::Round => lyon::tessellation::LineJoin::Round,
        LineJoin::Bevel => lyon::tessellation::LineJoin::Bevel,
    }
}

fn into_line_cap(line_cap: LineCap) -> lyon::tessellation::LineCap {
    match line_cap {
        LineCap::Butt => lyon::tessellation::LineCap::Butt,
        LineCap::Square => lyon::tessellation::LineCap::Square,
        LineCap::Round => lyon::tessellation::LineCap::Round,
    }
}

fn into_fill_rule(rule: FillRule) -> lyon::tessellation::FillRule {
    match rule {
        FillRule::NonZero => lyon::tessellation::FillRule::NonZero,
        FillRule::EvenOdd => lyon::tessellation::FillRule::EvenOdd,
    }
}

pub fn dashed(path: &Path, line_dash: LineDash<'_>) -> Path {
    use lyon::algorithms::walk::{walk_along_path, RepeatedPattern, WalkerEvent};
    use lyon::path::iterator::PathIterator;

    let mut dashed_path = PathBuilder::new();

    let segments_odd = (line_dash.segments.len() % 2 == 1)
        .then(|| [line_dash.segments, line_dash.segments].concat());

    let mut draw_line = false;

    walk_along_path(
        path.raw.iter().flattened(0.01),
        0.0,
        lyon::tessellation::StrokeOptions::DEFAULT_TOLERANCE,
        &mut RepeatedPattern {
            callback: |event: WalkerEvent<'_>| {
                let point = Point::new(event.position.x, event.position.y);

                if draw_line {
                    dashed_path.raw.line_to(point);
                } else {
                    dashed_path.raw.move_to(point);
                }

                draw_line = !draw_line;

                true
            },
            index: line_dash.offset,
            intervals: segments_odd.as_deref().unwrap_or(line_dash.segments),
        },
    );

    dashed_path.build()
}
