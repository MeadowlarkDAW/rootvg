use rootvg_quad::QuadFlags;
use smallvec::smallvec;
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

use rootvg::gradient::{LinearGradient, PackedGradient};
use rootvg::image::{ImagePrimitive, RcTexture};
use rootvg::math::{
    radians, Angle, PhysicalSizeI32, Point, PointI32, Rect, RectI32, Scale, ScaleFactor, Size,
    SizeI32, Vector,
};
use rootvg::msaa::Antialiasing;
use rootvg::quad::{
    Border, GradientQuad, GradientQuadPrimitive, Radius, SolidQuad, SolidQuadPrimitive,
};
use rootvg::tessellation::{
    path::{ArcPath, PathBuilder},
    stroke::{LineCap, LineDash, LineJoin, Stroke},
    Tessellator,
};
use rootvg::text::{Metrics, RcTextBuffer, TextPrimitive, TextProperties};
use rootvg::{
    color::{hex, PackedSrgb, RGBA8},
    surface::DefaultSurface,
    Canvas,
};
use rootvg_mesh::MeshPrimitive;
use rootvg_text::{svg::SvgIconSystem, ContentType, CustomGlyphDesc, FontSystem};

const WINDOW_SIZE: (f32, f32) = (800.0, 425.0);

static SVG_ICON: &[u8] = include_bytes!("./music.svg");

fn main() {
    // Set up logging stuff
    let env = env_logger::Env::default().filter_or("LOG_LEVEL", "info");
    env_logger::init_from_env(env);

    let event_loop = EventLoop::new().unwrap();
    event_loop
        .run_app(&mut DemoApp {
            state: None,
            font_system: FontSystem::new(),
            svg_icon_system: SvgIconSystem::default(),
        })
        .unwrap();
}

struct State {
    window: Arc<Window>,
    physical_size: PhysicalSizeI32,
    scale_factor: ScaleFactor,
    surface: DefaultSurface<'static>,
    canvas: Canvas,
    clear_color: PackedSrgb,
    primitives: MyPrimitives,
}

struct MyPrimitives {
    solid_quad: SolidQuadPrimitive,
    gradient_quad: GradientQuadPrimitive,
    text_primitive: TextPrimitive,
    svg_icon_primitive: TextPrimitive,
    image_primitive: ImagePrimitive,
    arc_mesh: MeshPrimitive,
    rect_mesh: MeshPrimitive,
    bezier_mesh: MeshPrimitive,
}

struct DemoApp {
    state: Option<State>,
    font_system: FontSystem,
    svg_icon_system: SvgIconSystem,
}

impl DemoApp {
    fn create_window(&mut self, event_loop: &ActiveEventLoop) {
        // --- Set up winit window -----------------------------------------------------------

        let window_attributes = Window::default_attributes()
            .with_inner_size(winit::dpi::LogicalSize::new(
                WINDOW_SIZE.0 as f64,
                WINDOW_SIZE.1 as f64,
            ))
            .with_title("RootVG Demo");
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        let physical_size = window.inner_size();
        // RootVG uses integers to represent physical pixels instead of unsigned integers.
        let physical_size =
            PhysicalSizeI32::new(physical_size.width as i32, physical_size.height as i32);
        let scale_factor: ScaleFactor = window.scale_factor().into();

        // --- Surface -----------------------------------------------------------------------

        // RootVG provides an optional default wgpu surface configuration for convenience.
        let surface = rootvg::surface::DefaultSurface::new(
            physical_size,
            scale_factor,
            Arc::clone(&window),
            rootvg::surface::DefaultSurfaceConfig {
                // Anti-aliasing can be used to smooth out mesh primitives. This has no
                // effect on other primitive types.
                antialiasing: Some(Antialiasing::MSAAx8),
                ..Default::default()
            },
        )
        .unwrap();

        // --- Color format ------------------------------------------------------------------

        // RootVG uses colors in a packed SRGB format of `[f32; 4]`.
        //
        // This is to prevent the need to constantly convert from an 8-bit RGBA
        // representation to the representation used by the GPU.
        let clear_color: PackedSrgb = RGBA8::new(15, 15, 15, 255).into();

        // --- Canvas ------------------------------------------------------------------------

        // A `Canvas` automatically batches primitives and renders them to a
        // render target (such as the output framebuffer).
        let canvas = rootvg::Canvas::new(
            &surface.device,
            &surface.queue,
            surface.format(),
            surface.canvas_config(),
            &mut self.font_system,
        );

        let primitives = Self::create_primitives(&mut self.font_system);

        // --- Load SVG icons ----------------------------------------------------------------

        self.svg_icon_system
            .add_from_bytes(0u16, SVG_ICON, &Default::default(), ContentType::Mask)
            .unwrap();

        self.state = Some(State {
            window,
            physical_size,
            scale_factor,
            surface,
            canvas,
            clear_color,
            primitives,
        });
    }

    fn create_primitives(font_system: &mut FontSystem) -> MyPrimitives {
        // --- Quads -------------------------------------------------------------------------

        // A solid quad draws a rounded rectangle with a solid background.
        let solid_quad: SolidQuadPrimitive = SolidQuad {
            bounds: Rect::new(Point::new(30.0, 30.0), Size::new(100.0, 100.0)),
            bg_color: RGBA8::new(30, 235, 150, 255).into(),
            border: Border {
                // The quad can have an outline filled with a solid color.
                color: RGBA8::new(19, 147, 94, 255).into(),
                width: 3.0,
                // A large radius turns this quad into a circle.
                radius: 50.0.into(),
            },
            flags: Default::default(),
        }
        .into();

        // A gradient quad draws a rounded rectangle with a gradient background.
        // This quad shows an example of using the builder pattern.
        let gradient_quad: GradientQuadPrimitive = GradientQuad::builder(Size::new(100.0, 200.0))
            .position(Point::new(300.0, 30.0))
            .bg_gradient(
                LinearGradient::new(radians(std::f32::consts::PI))
                    .add_stop(0.0, RGBA8::new(20, 0, 100, 255))
                    .add_stop(1.0, RGBA8::new(200, 0, 100, 255)),
            )
            .border_color(RGBA8::new(150, 150, 150, 255))
            .border_width(1.0)
            .border_radius(Radius {
                top_left: 20.0,
                top_right: 5.0,
                bottom_left: 0.0,
                bottom_right: 20.0,
            })
            .into();

        // --- Text --------------------------------------------------------------------------

        // First create a text buffer which performs layout and shaping on some text. This
        // is essentially a `cosmic-text` buffer.
        //
        // The `Rc` denotes that the buffer is wrapped in a shared reference-counted pointer.
        // This allows us to cheaply copy a pointer to clone a `TextPrimitive` instead of
        // cloning the whole buffer.
        let text_buffer = RcTextBuffer::new(
            "Hello World!",
            TextProperties {
                metrics: Metrics {
                    font_size: 14.0,
                    line_height: 20.0,
                },
                ..Default::default()
            },
            // The "bounds" denotes the visible area. Any text that lies outside of this
            // bounds wrapped/clipped depending on the text properties.
            None,
            None,
            false,
            font_system,
        );
        let text_primitive = TextPrimitive::new(
            text_buffer,
            Point::new(310.0, 100.0),
            // The `glyhpon` crate doesn't use our `PackedSrgb` format.
            RGBA8::new(200, 200, 200, 255),
            None,
        );

        // --- Svg Icon ----------------------------------------------------------------------

        let svg_icon_primitive = TextPrimitive::new_with_icons(
            None,
            Point::new(180.0, 220.0),
            // Convenience method for creating an RGBA8 color
            hex(0x9664c8),
            Some(Rect::from_size(Size::new(100.0, 100.0))),
            smallvec![CustomGlyphDesc {
                id: 0,
                left: 0.0,
                top: 0.0,
                color: None,
                size: 64.0,
                metadata: 0,
            }],
        );

        // --- Image -------------------------------------------------------------------------

        // Load an image into memory.
        let texture_bytes = include_bytes!("../assets/logo.png");
        let texture_raw = image::load_from_memory(texture_bytes).unwrap();

        // Construct an `RcTexture` which holds our image data for uploading to the GPU and
        // serves as a handle to the GPU texture.
        //
        // The `Rc` denotes that the buffer is wrapped in a shared reference-counted pointer.
        // This allows us to cheaply copy a pointer to clone an `ImagePrimitive` instead of
        // cloning the whole buffer.
        //
        // Once a texture is uploaded to the GPU, it automatically unloads its image data
        // from RAM.
        let texture = RcTexture::new(texture_raw.to_rgba8());

        // Construct an `ImagePrimitive`.
        let image_primitive = ImagePrimitive::builder(texture)
            .position(Point::new(500.0, 50.0))
            // Images can have a transformation applied to them.
            .rotation(Angle { radians: 0.3 }, Point::new(0.5, 0.5))
            .scale(Scale::new(0.5), Scale::new(0.5))
            .build();

        // --- Meshes & tessellation ---------------------------------------------------------

        // The `lyon` crate can be used to generate meshes.

        // Construct a path that a stroke or a fill can be applied to.
        let arc_path = PathBuilder::new()
            .arc(ArcPath {
                center: Point::new(50.0, 50.0),
                radius: 25.0,
                start_angle: radians(std::f32::consts::PI * 1.5 - 2.4),
                end_angle: radians(std::f32::consts::PI * 1.5 + 2.4),
            })
            .build();

        let stroke = Stroke {
            style: RGBA8::new(0, 200, 255, 255).into(),
            width: 5.0,
            line_cap: LineCap::Round,
            line_join: LineJoin::default(),
            line_dash: LineDash::default(),
        };

        // A tessellator generates mesh primitives.
        // In this case we only apply one stroke/fill operation, so we can
        // use the method that generates just a single mesh primitive.
        let arc_mesh = Tessellator::new()
            .stroke(&arc_path, stroke)
            .into_primitive()
            .unwrap();

        let rect_path = PathBuilder::new()
            .rectangle(Point::new(0.0, 0.0), Size::new(10.0, 50.0))
            .build();

        let mut rect_mesh = Tessellator::new()
            // Transformations can be applied before tessellation.
            .rotate(radians(0.1))
            .fill(&rect_path, RGBA8::new(0, 200, 255, 255))
            .into_primitive()
            .unwrap();

        // Transformations can also be applied after tessellation. This can
        // be useful if you need to repeatedly transform a complex mesh.
        rect_mesh.set_rotation(radians(0.3), Point::new(5.0, 25.0));

        let gradient_stroke = Stroke {
            // Fill the line with a gradient this time.
            style: PackedGradient::new(
                &LinearGradient::new(radians(std::f32::consts::PI))
                    .add_stop(0.0, RGBA8::new(0, 100, 200, 255))
                    .add_stop(1.0, RGBA8::new(200, 0, 100, 255))
                    .into(),
                Rect::new(Point::new(0.0, 0.0), Size::new(100.0, 100.0)),
            )
            .into(),
            width: 5.0,
            line_cap: LineCap::Round,
            line_join: LineJoin::default(),
            line_dash: LineDash::default(),
        };

        let bezier_path = PathBuilder::new()
            .move_to(Point::new(0.0, 0.0))
            .bezier_curve_to(
                Point::new(0.0, 100.0),
                Point::new(100.0, 0.0),
                Point::new(100.0, 100.0),
            )
            .bezier_curve_to(
                Point::new(100.0, 100.0),
                Point::new(200.0, 100.0),
                Point::new(200.0, 50.0),
            )
            .build();

        let bezier_mesh = Tessellator::new()
            .stroke(&bezier_path, gradient_stroke)
            .into_primitive()
            .unwrap();

        MyPrimitives {
            solid_quad,
            gradient_quad,
            text_primitive,
            svg_icon_primitive,
            image_primitive,
            arc_mesh,
            rect_mesh,
            bezier_mesh,
        }
    }
}

impl ApplicationHandler for DemoApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_none() {
            self.create_window(event_loop);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(state) = &mut self.state else {
            return;
        };

        match event {
            WindowEvent::RedrawRequested => {
                {
                    // A `CanvasContext` is used to add primitives to the canvas.
                    //
                    // Each time `canvas.begin()` is called, all primitives that were
                    // previously added are cleared. Primitives are designed to
                    // be cheap to clone.
                    let mut cx = state.canvas.begin(state.physical_size, state.scale_factor);

                    // At any point the "z index" can be changed.
                    //
                    // Note that `canvas.begin()` resets the z index to `0`.
                    cx.set_z_index(1);

                    // Primitives with the same z index are *NOT* gauranteed to be
                    // drawn in the same order that they are added to the canvas.
                    // Because of that, we need add the text primitive with a higher
                    // z index.
                    cx.add(state.primitives.text_primitive.clone());
                    cx.add(state.primitives.svg_icon_primitive.clone());

                    cx.set_z_index(0);

                    cx.add(state.primitives.solid_quad);
                    cx.add(state.primitives.gradient_quad);
                    cx.add(state.primitives.image_primitive.clone());

                    // Primitives can also be constructed inline. This is a bit less
                    // efficient, but is more convenient.
                    cx.add(
                        SolidQuad::builder(Size::new(50.0, 60.0))
                            .position(Point::new(163.5, 100.5))
                            .bg_color(PackedSrgb::TRANSPARENT)
                            .border_color(RGBA8::new(150, 150, 150, 255))
                            .border_width(1.8)
                            .flags(QuadFlags::SNAP_ALL_TO_NEAREST_PIXEL)
                            .build(),
                    );

                    // A scissoring rectangle can be used.
                    cx.set_scissor_rect(RectI32::new(
                        PointI32::new(50, 150),
                        SizeI32::new(100, 100),
                    ));

                    cx.add_with_offset(state.primitives.solid_quad, Vector::new(0.0, 150.0));

                    // Calling this will reset the scissoring rectangle to cover the
                    // whole canvas.
                    cx.reset_scissor_rect();

                    // Primitives can also be added with an offset applied.
                    //
                    // This can be useful to create a bunch of copies of the same
                    // primitive.
                    cx.add_with_offset(
                        state.primitives.arc_mesh.clone(),
                        Vector::new(100.0, 300.0),
                    );
                    cx.add_with_offset(
                        state.primitives.arc_mesh.clone(),
                        Vector::new(200.0, 300.0),
                    );
                    cx.add_with_offset(
                        state.primitives.rect_mesh.clone(),
                        Vector::new(340.0, 325.0),
                    );
                    cx.add_with_offset(
                        state.primitives.bezier_mesh.clone(),
                        Vector::new(400.0, 300.0),
                    );
                }

                // Set up the frame and wgpu encoder.
                let frame = state.surface.get_current_texture().unwrap();
                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                let mut encoder = state
                    .surface
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                // Render the canvas to the target texture.
                state
                    .canvas
                    .render_to_target(
                        Some(state.clear_color),
                        &state.surface.device,
                        &state.surface.queue,
                        &mut encoder,
                        &view,
                        state.physical_size,
                        &mut self.font_system,
                        &mut self.svg_icon_system,
                    )
                    .unwrap();

                state.window.pre_present_notify();

                // Submit the commands and present the frame.
                state.surface.queue.submit(Some(encoder.finish()));
                frame.present();
            }
            // Resize the Canvas to match the new window size
            WindowEvent::Resized(new_size) => {
                state.physical_size =
                    PhysicalSizeI32::new(new_size.width as i32, new_size.height as i32);
                state
                    .surface
                    .resize(state.physical_size, state.scale_factor);
                state.window.request_redraw();
            }
            WindowEvent::ScaleFactorChanged {
                scale_factor: new_scale,
                inner_size_writer: _,
            } => {
                state.scale_factor = new_scale.into();
                state
                    .surface
                    .resize(state.physical_size, state.scale_factor);
                state.window.request_redraw();
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            _ => {}
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        // Be sure to drop the wgpu surface before the window closes,
        // or else the program might segfault.
        self.state = None;
    }
}
