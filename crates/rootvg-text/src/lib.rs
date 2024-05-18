//use once_cell::sync::OnceCell;
//use std::sync::RwLock;

pub use glyphon;

pub use glyphon::cosmic_text::Align;
pub use glyphon::{Attrs, Family, FamilyOwned, Metrics, Shaping, Stretch, Style, Weight, Wrap};

mod buffer;
mod primitive;
mod properties;

pub mod pipeline;

pub use buffer::RcTextBuffer;
pub use primitive::TextPrimitive;
pub use properties::TextProperties;

/*
/// Returns the global [`FontSystem`].
pub fn font_system() -> &'static RwLock<FontSystem> {
    static FONT_SYSTEM: OnceCell<RwLock<FontSystem>> = OnceCell::new();

    FONT_SYSTEM.get_or_init(|| {
        RwLock::new(FontSystem {
            raw: glyphon::FontSystem::new(),
        })
    })
}

static WRITE_LOCK_PANIC_MSG: &'static str = "Failed to obtain write lock on font system";

/// A set of system fonts.
pub struct FontSystem {
    raw: glyphon::FontSystem,
}

impl FontSystem {
    /// Returns the raw [`glyphon::FontSystem`].
    pub fn raw(&self) -> &glyphon::FontSystem {
        &self.raw
    }

    /// Returns the raw [`glyphon::FontSystem`].
    pub fn raw_mut(&mut self) -> &mut glyphon::FontSystem {
        &mut self.raw
    }
}
*/