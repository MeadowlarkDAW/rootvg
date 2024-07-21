use std::path::Path;

use glyphon::{ContentType, CustomGlyphInput, CustomGlyphOutput};
use resvg::{tiny_skia::Pixmap, usvg::Transform};
use rustc_hash::FxHashMap;

// Re-export resvg for convenience.
pub use resvg;

use crate::CustomGlyphID;

/// A system for loading, parsing, and rastering SVG icons
#[derive(Default)]
pub struct SvgIconSystem {
    svgs: FxHashMap<CustomGlyphID, SvgData>,
}

impl SvgIconSystem {
    /// Add an svg source to from an [`resvg::usvg::Tree`].
    ///
    /// * id - A unique identifier for this resource.
    /// * tree - The parsed SVG data.
    /// * is_symbolic - If `true`, then only the alpha channel will be used and the icon can
    /// be filled with any solid color. If `false`, then the icon will be rendered in full
    /// color.
    pub fn add_from_tree(
        &mut self,
        id: impl Into<CustomGlyphID>,
        tree: resvg::usvg::Tree,
        content_type: ContentType,
    ) {
        self.svgs.insert(id.into(), SvgData { tree, content_type });
    }

    /// Add an svg source from raw bytes.
    ///
    /// * id - A unique identifier for this resource
    /// * data - The raw SVG file as bytes
    /// * opts - Additional options for parsing the SVG file
    /// * is_symbolic - If `true`, then only the alpha channel will be used and the icon can
    /// be filled with any solid color. If `false`, then the icon will be rendered in full
    /// color.
    pub fn add_from_bytes(
        &mut self,
        id: impl Into<CustomGlyphID>,
        data: &[u8],
        opt: &resvg::usvg::Options<'_>,
        content_type: ContentType,
    ) -> Result<(), resvg::usvg::Error> {
        let tree = resvg::usvg::Tree::from_data(data, opt)?;
        self.add_from_tree(id, tree, content_type);
        Ok(())
    }

    /// Add an svg source from a string.
    ///
    /// * id - A unique identifier for this resource
    /// * str - The SVG data as a string
    /// * opts - Additional options for parsing the SVG file
    /// * is_symbolic - If `true`, then only the alpha channel will be used and the icon can
    /// be filled with any solid color. If `false`, then the icon will be rendered in full
    /// color.
    pub fn add_from_str(
        &mut self,
        id: impl Into<CustomGlyphID>,
        text: &str,
        opt: &resvg::usvg::Options<'_>,
        content_type: ContentType,
    ) -> Result<(), resvg::usvg::Error> {
        let tree = resvg::usvg::Tree::from_str(text, opt)?;
        self.add_from_tree(id, tree, content_type);
        Ok(())
    }

    /// Add an svg source from a file path.
    ///
    /// * id - A unique identifier for this resource
    /// * path - The path to the SVG file
    /// * opts - Additional options for parsing the SVG file
    /// * is_symbolic - If `true`, then only the alpha channel will be used and the icon can
    /// be filled with any solid color. If `false`, then the icon will be rendered in full
    /// color.
    pub fn add_from_path(
        &mut self,
        id: impl Into<CustomGlyphID>,
        path: &Path,
        opt: &resvg::usvg::Options<'_>,
        content_type: ContentType,
    ) -> Result<(), LoadSvgError> {
        let data = std::fs::read(path)?;
        let tree = resvg::usvg::Tree::from_data(&data, opt)?;
        self.add_from_tree(id, tree, content_type);
        Ok(())
    }

    // Returns `true` if the source was removed, or `false` if there was
    // no source with that ID.
    pub fn remove(&mut self, id: impl Into<CustomGlyphID>) -> bool {
        self.svgs.remove(&id.into()).is_some()
    }

    /// Rasterize the SVG icon.
    pub fn render_custom_glyph(&mut self, input: CustomGlyphInput) -> Option<CustomGlyphOutput> {
        let Some(svg_data) = self.svgs.get(&input.id) else {
            return None;
        };

        let svg_size = svg_data.tree.size();
        let max_side_len = svg_size.width().max(svg_size.height());

        let should_rasterize = max_side_len > 0.0;

        let (scale, width, height, pixmap) = if should_rasterize {
            let glyph_size = input.size * input.scale;
            let scale = glyph_size / max_side_len;
            let width = (svg_size.width() * scale).ceil();
            let height = (svg_size.height() * scale).ceil();

            if width <= 0.0 || height <= 0.0 {
                (0.0, 0, 0, None)
            } else if let Some(pixmap) = Pixmap::new(width as u32, height as u32) {
                (scale, width as u32, height as u32, Some(pixmap))
            } else {
                (0.0, 0, 0, None)
            }
        } else {
            (0.0, 0, 0, None)
        };

        if let Some(mut pixmap) = pixmap {
            let mut transform = Transform::from_scale(scale, scale);

            let offset_x = input.x_bin.as_float();
            let offset_y = input.y_bin.as_float();

            if offset_x != 0.0 || offset_y != 0.0 {
                transform = transform.post_translate(offset_x, offset_y);
            }

            resvg::render(&svg_data.tree, transform, &mut pixmap.as_mut());

            let data: Vec<u8> = if let ContentType::Mask = svg_data.content_type {
                // Only use the alpha channel for symbolic icons.
                pixmap.data().iter().skip(3).step_by(4).copied().collect()
            } else {
                pixmap.data().to_vec()
            };

            Some(CustomGlyphOutput {
                data,
                width,
                height,
                content_type: svg_data.content_type,
            })
        } else {
            None
        }
    }
}

#[derive(Clone)]
struct SvgData {
    tree: resvg::usvg::Tree,
    content_type: ContentType,
}

/// An error occured while loading an SVG file
#[derive(Debug, thiserror::Error)]
pub enum LoadSvgError {
    #[error("Error loading svg file: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Error parsing svg file: {0}")]
    ParseError(#[from] resvg::usvg::Error),
}
