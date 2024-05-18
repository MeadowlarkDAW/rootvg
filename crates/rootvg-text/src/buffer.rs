use glyphon::cosmic_text::Align;
use glyphon::FontSystem;
use std::cell::{Ref, RefCell};
use std::rc::Rc;

use rootvg_core::math::Size;

use super::TextProperties;

#[derive(Debug)]
struct TextBufferInner {
    raw_buffer: glyphon::Buffer,
    props: TextProperties,
    bounds_size: Size,
    has_text: bool,
}

#[derive(Debug)]
pub struct RcTextBuffer {
    inner: Rc<RefCell<TextBufferInner>>,
    /// Used to quickly diff text primitives for changes.
    generation: u64,
}

impl RcTextBuffer {
    pub fn new(text: &str, props: TextProperties, bounds_size: Size, font_system: &mut FontSystem) -> Self {
        let mut raw_buffer = glyphon::Buffer::new(font_system, props.metrics);

        raw_buffer.set_size(font_system, bounds_size.width, bounds_size.height);
        raw_buffer.set_wrap(font_system, props.wrap);
        raw_buffer.set_text(font_system, text, props.attrs, props.shaping);

        let has_text = !text.is_empty();
        if has_text {
            shape(&mut raw_buffer, font_system, props.align);
        }

        Self {
            inner: Rc::new(RefCell::new(TextBufferInner {
                raw_buffer,
                props,
                bounds_size,
                has_text,
            })),
            generation: 0,
        }
    }

    pub fn bounds_size(&self) -> Size {
        RefCell::borrow(&self.inner).bounds_size
    }

    pub fn props<'a>(&'a self) -> Ref<'a, TextProperties> {
        let inner = RefCell::borrow(&self.inner);
        Ref::map(inner, |inner| &inner.props)
    }

    /// The minimum size (in logical points) needed to fit the text contents.
    pub fn measure(&self) -> Size {
        let inner = RefCell::borrow(&self.inner);
        let buffer = &inner.raw_buffer;

        let (width, total_lines) = buffer
            .layout_runs()
            .fold((0.0, 0usize), |(width, total_lines), run| {
                (run.line_w.max(width), total_lines + 1)
            });

        Size::new(width, total_lines as f32 * buffer.metrics().line_height)
    }

    pub fn set_text_and_props(&mut self, text: &str, props: TextProperties, font_system: &mut FontSystem) {
        let mut inner = RefCell::borrow_mut(&self.inner);

        if inner.props.metrics != props.metrics {
            inner
                .raw_buffer
                .set_metrics(font_system, props.metrics)
        }

        if inner.props.wrap != props.wrap {
            inner.raw_buffer.set_wrap(font_system, props.wrap);
        }

        inner
            .raw_buffer
            .set_text(font_system, text, props.attrs, props.shaping);

        inner.has_text = !text.is_empty();

        if inner.has_text {
            shape(&mut inner.raw_buffer, font_system, props.align);
        }

        inner.props = props;

        self.generation += 1;
    }

    pub fn set_text(&mut self, text: &str, font_system: &mut FontSystem) {
        let mut inner = RefCell::borrow_mut(&self.inner);
        let TextBufferInner {
            raw_buffer,
            props,
            bounds_size: _,
            has_text,
        } = &mut *inner;

        raw_buffer.set_text(font_system, text, props.attrs, props.shaping);

        *has_text = !text.is_empty();

        if *has_text {
            shape(raw_buffer, font_system, props.align);
        }

        self.generation += 1;
    }

    /// Set the bounds of the text in logical points.
    pub fn set_bounds(&mut self, bounds_size: Size, font_system: &mut FontSystem) {
        let mut inner = RefCell::borrow_mut(&self.inner);
        let TextBufferInner {
            raw_buffer,
            props,
            bounds_size: inner_bounds_size,
            has_text,
        } = &mut *inner;

        if *inner_bounds_size == bounds_size {
            return;
        }
        *inner_bounds_size = bounds_size;

        raw_buffer.set_size(
            font_system,
            bounds_size.width as f32,
            bounds_size.height as f32,
        );

        if *has_text {
            shape(raw_buffer, font_system, props.align);
        }

        self.generation += 1;
    }

    pub(crate) fn raw_buffer<'a>(&'a self) -> Ref<'a, glyphon::Buffer> {
        let inner = RefCell::borrow(&self.inner);
        Ref::map(inner, |inner| &inner.raw_buffer)
    }
}

impl Clone for RcTextBuffer {
    fn clone(&self) -> Self {
        Self {
            inner: Rc::clone(&self.inner),
            generation: self.generation,
        }
    }
}

impl PartialEq for RcTextBuffer {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner) && self.generation == other.generation
    }
}

fn shape(
    buffer: &mut glyphon::Buffer,
    font_system: &mut glyphon::FontSystem,
    align: Option<Align>,
) {
    for line in buffer.lines.iter_mut() {
        if line.align() != align {
            line.set_align(align);
        }
    }

    buffer.shape_until_scroll(font_system, true);
}
