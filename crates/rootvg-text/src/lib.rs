//use once_cell::sync::OnceCell;
//use std::sync::RwLock;

mod buffer;
mod primitive;
mod properties;

pub mod pipeline;

#[cfg(feature = "svg-icons")]
pub mod svg;

pub use glyphon;

pub use glyphon::cosmic_text::Align;
pub use glyphon::{
    Attrs, ContentType, Family, FamilyOwned, FontSystem, Metrics, Shaping, Stretch, Style, Weight,
    Wrap,
};

pub use buffer::{EditorBorrowStatus, RcTextBuffer};
pub use primitive::TextPrimitive;
pub use properties::TextProperties;

#[cfg(feature = "svg-icons")]
pub use glyphon::{CustomGlyph, CustomGlyphId};
