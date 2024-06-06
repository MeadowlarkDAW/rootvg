//use once_cell::sync::OnceCell;
//use std::sync::RwLock;

pub use glyphon;

pub use glyphon::cosmic_text::Align;
pub use glyphon::{Attrs, Family, FamilyOwned, Metrics, Shaping, Stretch, Style, Weight, Wrap};

mod buffer;
mod primitive;
mod properties;

pub mod pipeline;

pub use buffer::{EditorBorrowStatus, RcTextBuffer};
pub use primitive::TextPrimitive;
pub use properties::TextProperties;
