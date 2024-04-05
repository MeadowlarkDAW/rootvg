use bytemuck::{Pod, Zeroable};
use rootvg_core::math::{Angle, Point, Rect, Scale, Size, Transform, Vector};

use crate::texture::RcTexture;

#[derive(Debug, Clone, PartialEq)]
pub struct ImagePrimitive {
    pub texture: RcTexture,
    pub vertex: ImageVertex,
}

impl ImagePrimitive {
    pub fn new(texture: RcTexture, position: Point) -> Self {
        let size = texture.size();

        Self {
            texture,
            vertex: ImageVertex {
                position: position.into(),
                size: [size.width as f32, size.height as f32],
                ..Default::default()
            },
        }
    }

    pub fn builder(texture: RcTexture) -> ImagePrimitiveBuilder {
        ImagePrimitiveBuilder::new(texture)
    }

    pub fn position(&self) -> Point {
        Point::new(self.vertex.position[0], self.vertex.position[1])
    }

    pub fn set_position(&mut self, position: Point) {
        self.vertex.position = position.into();
    }
}

pub struct ImagePrimitiveBuilder {
    primitive: ImagePrimitive,
}

impl ImagePrimitiveBuilder {
    pub fn new(texture: RcTexture) -> Self {
        Self {
            primitive: ImagePrimitive::new(texture, Point::default()),
        }
    }

    /// The position of the top-left corner of the image (before rotation) in logical points.
    pub fn position(mut self, position: Point) -> Self {
        self.primitive.vertex.position = position.into();
        self
    }

    pub fn scale(mut self, scale_x: Scale, scale_y: Scale) -> Self {
        self.primitive.vertex.size[0] *= scale_x.0;
        self.primitive.vertex.size[1] *= scale_y.0;
        self
    }

    pub fn rotation(mut self, angle: Angle, origin_normal: Point) -> Self {
        let transform = Transform::translation(-origin_normal.x, -origin_normal.y)
            .then_rotate(angle)
            .then_translate(Vector::new(origin_normal.x, origin_normal.y));

        self.primitive.vertex.transform = transform.to_array();
        self.primitive.vertex.has_transform = 1;
        self
    }

    pub fn transform(mut self, transform: Transform) -> Self {
        self.primitive.vertex.transform = transform.to_array();
        self.primitive.vertex.has_transform = 1;
        self
    }

    pub fn unnormalized_uv_rect(mut self, uv_rect: Rect) -> Self {
        let normal_uv_rect = normalized_uv_rect(
            uv_rect,
            Size::new(self.primitive.vertex.size[0], self.primitive.vertex.size[1]),
        );

        self.primitive.vertex.normalized_uv_pos = normal_uv_rect.origin.into();
        self.primitive.vertex.normalized_uv_size = normal_uv_rect.size.into();
        self
    }

    pub fn normalized_uv_rect(mut self, uv_rect: Rect) -> Self {
        self.primitive.vertex.normalized_uv_pos = uv_rect.origin.into();
        self.primitive.vertex.normalized_uv_size = uv_rect.size.into();
        self
    }

    pub fn build(self) -> ImagePrimitive {
        self.primitive
    }
}

impl From<ImagePrimitiveBuilder> for ImagePrimitive {
    fn from(i: ImagePrimitiveBuilder) -> Self {
        i.build()
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct ImageVertex {
    /// The position of the top-left corner of the [`Image`] (before rotation)
    /// in logical points.
    pub position: [f32; 2],

    /// The size of the [`Image`] in logical points.
    pub size: [f32; 2],

    /// The position of the top-left uv coordinate in the texture, normalized to
    /// therange `[0.0, 1.0]`
    ///
    /// By default this is set to `[0.0, 0.0]`
    pub normalized_uv_pos: [f32; 2],

    /// The size of the rect in the texture, normalized to the range `[0.0, 1.0]`
    ///
    /// By default this is set to `[1.0, 1.0]`
    pub normalized_uv_size: [f32; 2],

    /// A 2d transform represented by a column-major 3 by 3 matrix, compressed down
    /// to 3 by 2.
    ///
    /// Note that `size` is not included in the `transform`.
    pub transform: [f32; 6],

    /// Whether or not to apply the `transform` matrix. This is used to optimize
    /// images with no transformations.
    ///
    /// Note that `size` is not included in the `transform`.
    ///
    /// By default this is set to `0` (false).
    pub has_transform: u32,
}

impl Default for ImageVertex {
    fn default() -> Self {
        Self {
            position: [0.0; 2],
            size: [0.0; 2],
            normalized_uv_pos: [0.0; 2],
            normalized_uv_size: [1.0; 2],
            transform: [0.0; 6],
            has_transform: 0,
        }
    }
}

fn normalized_uv_rect(uv_rect: Rect, texture_size: Size) -> Rect {
    Rect::new(
        Point::new(
            uv_rect.origin.x / texture_size.width,
            uv_rect.origin.y / texture_size.height,
        ),
        Size::new(
            uv_rect.size.width / texture_size.width,
            uv_rect.size.height / texture_size.height,
        ),
    )
}
