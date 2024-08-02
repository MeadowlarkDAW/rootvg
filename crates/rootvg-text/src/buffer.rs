use glyphon::cosmic_text::{Align, BufferRef};
use glyphon::{Edit, FontSystem};
use std::cell::{Ref, RefCell};
use std::fmt::Debug;
use std::hint::unreachable_unchecked;
use std::rc::Rc;

use rootvg_core::math::Size;

use super::TextProperties;

pub enum BufferType {
    Normal(glyphon::Buffer),
    Editor(glyphon::Editor<'static>),
}

impl BufferType {
    pub fn editor(&self) -> Option<&glyphon::Editor<'static>> {
        if let BufferType::Editor(editor) = self {
            Some(editor)
        } else {
            None
        }
    }
}

impl Debug for BufferType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BufferType::Normal(_) => write!(f, "BufferType::Normal"),
            BufferType::Editor(_) => write!(f, "BufferType::Editor"),
        }
    }
}

impl BufferType {
    pub fn raw(&self) -> &glyphon::Buffer {
        match self {
            BufferType::Normal(b) => b,
            BufferType::Editor(editor) => {
                if let BufferRef::Owned(b) = editor.buffer_ref() {
                    b
                } else {
                    unreachable!()
                }
            }
        }
    }

    pub fn raw_mut(&mut self) -> &mut glyphon::Buffer {
        match self {
            BufferType::Normal(b) => b,
            BufferType::Editor(editor) => {
                if let BufferRef::Owned(b) = editor.buffer_ref_mut() {
                    b
                } else {
                    unreachable!()
                }
            }
        }
    }
}

#[derive(Debug)]
struct TextBufferInner {
    raw_buffer: BufferType,
    props: TextProperties,
    bounds_width: Option<f32>,
    bounds_height: Option<f32>,
    has_text: bool,
}

#[derive(Debug)]
pub struct RcTextBuffer {
    inner: Rc<RefCell<TextBufferInner>>,
    /// Used to quickly diff text primitives for changes.
    generation: u64,
}

impl RcTextBuffer {
    pub fn new(
        text: &str,
        props: TextProperties,
        bounds_width: Option<f32>,
        bounds_height: Option<f32>,
        is_editor: bool,
        font_system: &mut FontSystem,
    ) -> Self {
        let mut raw_buffer = glyphon::Buffer::new(font_system, props.metrics);

        raw_buffer.set_size(font_system, bounds_width, bounds_height);
        raw_buffer.set_wrap(font_system, props.wrap);
        raw_buffer.set_text(font_system, text, props.attrs, props.shaping);

        let has_text = !text.is_empty();
        if has_text {
            shape(&mut raw_buffer, font_system, props.align);
        }

        let raw_buffer = if is_editor {
            BufferType::Editor(glyphon::Editor::new(raw_buffer))
        } else {
            BufferType::Normal(raw_buffer)
        };

        Self {
            inner: Rc::new(RefCell::new(TextBufferInner {
                raw_buffer,
                props,
                bounds_width,
                bounds_height,
                has_text,
            })),
            generation: 0,
        }
    }

    pub fn bounds_width(&self) -> Option<f32> {
        RefCell::borrow(&self.inner).bounds_width
    }

    pub fn bounds_height(&self) -> Option<f32> {
        RefCell::borrow(&self.inner).bounds_height
    }

    pub fn props(&self) -> Ref<'_, TextProperties> {
        let inner = RefCell::borrow(&self.inner);
        Ref::map(inner, |inner| &inner.props)
    }

    /// The minimum size (in logical points) needed to fit the text contents.
    pub fn measure(&self) -> Size {
        let inner = RefCell::borrow(&self.inner);
        let buffer = inner.raw_buffer.raw();

        let (width, total_lines) = buffer
            .layout_runs()
            .fold((0.0, 0usize), |(width, total_lines), run| {
                (run.line_w.max(width), total_lines + 1)
            });

        Size::new(width, total_lines as f32 * buffer.metrics().line_height)
    }

    pub fn set_text_and_props(
        &mut self,
        text: &str,
        new_props: TextProperties,
        font_system: &mut FontSystem,
    ) {
        let mut inner = RefCell::borrow_mut(&self.inner);
        let TextBufferInner {
            raw_buffer,
            props,
            has_text,
            ..
        } = &mut *inner;

        let raw_buffer = raw_buffer.raw_mut();

        if props.metrics != props.metrics {
            raw_buffer.set_metrics(font_system, props.metrics)
        }

        if props.wrap != props.wrap {
            raw_buffer.set_wrap(font_system, props.wrap);
        }

        raw_buffer.set_text(font_system, text, props.attrs, props.shaping);

        *has_text = !text.is_empty();

        if *has_text {
            shape(raw_buffer, font_system, props.align);
        }

        *props = new_props;

        self.generation += 1;
    }

    pub fn set_text(&mut self, text: &str, font_system: &mut FontSystem) {
        let mut inner = RefCell::borrow_mut(&self.inner);
        let TextBufferInner {
            raw_buffer,
            props,
            bounds_width: _,
            bounds_height: _,
            has_text,
        } = &mut *inner;

        let raw_buffer = raw_buffer.raw_mut();

        raw_buffer.set_text(font_system, text, props.attrs, props.shaping);

        *has_text = !text.is_empty();

        if *has_text {
            shape(raw_buffer, font_system, props.align);
        }

        self.generation += 1;
    }

    /// Set the bounds of the text in logical points.
    pub fn set_bounds(
        &mut self,
        bounds_width: Option<f32>,
        bounds_height: Option<f32>,
        font_system: &mut FontSystem,
    ) {
        let mut inner = RefCell::borrow_mut(&self.inner);
        let TextBufferInner {
            raw_buffer,
            props,
            bounds_width: inner_bounds_width,
            bounds_height: inner_bounds_height,
            has_text,
        } = &mut *inner;

        if *inner_bounds_width == bounds_width && *inner_bounds_height == bounds_height {
            return;
        }
        *inner_bounds_width = bounds_width;
        *inner_bounds_height = bounds_height;

        let raw_buffer = raw_buffer.raw_mut();

        raw_buffer.set_size(font_system, bounds_width, bounds_height);

        if *has_text {
            shape(raw_buffer, font_system, props.align);
        }

        self.generation += 1;
    }

    pub fn buffer(&self) -> Ref<'_, BufferType> {
        let inner = RefCell::borrow(&self.inner);
        Ref::map(inner, |inner| &inner.raw_buffer)
    }

    /// Borrow the editor mutably. If this buffer does not have an editor then
    /// this will do nothing.
    ///
    /// Note, don't mutate the buffer bounds or the text properties using this
    /// method or else the state will get out of sync. Also don't shape the
    /// text as this method will automatically do that for you.
    ///
    /// You may mutate the text inside of the buffer, just be sure to mark
    /// that the text contents have changed in the returned `EditorBorrowStatus`.
    pub fn with_editor_mut<
        F: FnOnce(&mut glyphon::Editor, &mut FontSystem) -> EditorBorrowStatus,
    >(
        &mut self,
        f: F,
        font_system: &mut FontSystem,
    ) {
        let mut inner = RefCell::borrow_mut(&self.inner);
        let TextBufferInner {
            raw_buffer,
            props,
            bounds_width: _,
            bounds_height: _,
            has_text,
        } = &mut *inner;

        if let BufferType::Editor(editor) = raw_buffer {
            let status = (f)(editor, font_system);

            if status.text_changed {
                *has_text = status.has_text;

                if *has_text {
                    let b = match editor.buffer_ref_mut() {
                        BufferRef::Owned(b) => b,
                        _ => unreachable!(),
                    };

                    shape(b, font_system, props.align);
                }

                self.generation += 1;
            }
        }
    }

    pub fn raw_buffer(&self) -> Ref<'_, glyphon::Buffer> {
        let inner = RefCell::borrow(&self.inner);
        Ref::map(inner, |inner| {
            match &inner.raw_buffer {
                BufferType::Normal(b) => b,
                BufferType::Editor(editor) => {
                    if let BufferRef::Owned(b) = editor.buffer_ref() {
                        b
                    } else {
                        // SAFETY: Because of the constructor, `TextBufferInner`
                        // can only have an editor with an owned buffer.
                        unsafe { unreachable_unchecked() }
                    }
                }
            }
        })
    }

    pub fn sync_state_from_editor(&mut self) {
        // TODO
        self.generation += 1;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorBorrowStatus {
    pub text_changed: bool,
    pub has_text: bool,
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
