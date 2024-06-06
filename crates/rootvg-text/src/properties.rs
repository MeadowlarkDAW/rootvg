use glyphon::cosmic_text::{Align, CacheKeyFlags};
use glyphon::{Attrs, Family, Metrics, Shaping, Stretch, Style, Weight, Wrap};

/// The style of a font
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextProperties {
    /// The metrics of the font (font size and line height)
    ///
    /// By default this is set to `Metrics { font_size: 14.0, line_height: 16.0  }`.
    pub metrics: Metrics,
    /// The text alignment
    ///
    /// Setting to `None` will use `Align::Right` for RTL lines, and `Align::Left`
    /// for LTR lines.
    ///
    /// By default this is set to `None`.
    pub align: Option<Align>,
    /// The text attributes
    ///
    /// By default this is set to:
    ///```
    ///Attrs {
    /// color_opt: None,
    /// family: Family::SansSerif,
    /// stretch: Stretch::Normal,
    /// style: Style::Normal,
    /// weight: Weight::NORMAL,
    /// metadata: 0,
    ///}
    /// ```
    pub attrs: Attrs<'static>,
    /// The text wrapping
    ///
    /// By default this is set to `Wrap::None`.
    pub wrap: Wrap,
    /// The text shaping
    ///
    /// By default this is set to `Shaping::Basic`.
    pub shaping: Shaping,
}

impl Default for TextProperties {
    fn default() -> Self {
        Self {
            metrics: Metrics {
                font_size: 14.0,
                line_height: 16.0,
            },
            align: None,
            attrs: Attrs {
                color_opt: None,
                family: Family::SansSerif,
                stretch: Stretch::Normal,
                style: Style::Normal,
                weight: Weight::NORMAL,
                metadata: 0,
                cache_key_flags: CacheKeyFlags::empty(),
            },
            wrap: Wrap::None,
            shaping: Shaping::Basic,
        }
    }
}
