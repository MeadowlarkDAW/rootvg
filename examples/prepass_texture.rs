use rootvg_text::{glyphon::FontSystem, svg::SvgIconSystem};
use std::sync::Arc;
use wgpu::PipelineCompilationOptions;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

use rootvg::math::{PhysicalSizeI32, Point, Rect, ScaleFactor, Size};
use rootvg::quad::{SolidQuad, SolidQuadPrimitive};
use rootvg::{
    color::{PackedSrgb, RGBA8},
    math::PhysicalSizeU32,
};
use rootvg::{
    image::{ImagePrimitive, RcTexture},
    surface::DefaultSurface,
    Canvas,
};

const WINDOW_SIZE: (f32, f32) = (800.0, 425.0);
const PREPASS_TEXTURE_SIZE: PhysicalSizeU32 = PhysicalSizeU32::new(200, 200);

static SHADER: &str = "
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(1 - i32(in_vertex_index)) * 0.9;
    let y = f32(i32(in_vertex_index & 1u) * 2 - 1) * 0.9;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.8, 0.1, 0.1, 1.0);
}
";

fn main() {
    // Set up logging stuff
    let env = env_logger::Env::default().filter_or("LOG_LEVEL", "info");
    env_logger::init_from_env(env);

    let event_loop = EventLoop::new().unwrap();
    event_loop
        .run_app(&mut PrepassTextureApp {
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

    prepass_pipeline: wgpu::RenderPipeline,
    prepass_texture: wgpu::Texture,
    image_primitive: ImagePrimitive,
}

struct PrepassTextureApp {
    state: Option<State>,
    font_system: FontSystem,
    svg_icon_system: SvgIconSystem,
}

impl PrepassTextureApp {
    fn create_window(&mut self, event_loop: &ActiveEventLoop) {
        // --- Set up winit window -----------------------------------------------------------

        let window_attributes = Window::default_attributes()
            .with_inner_size(winit::dpi::LogicalSize::new(
                WINDOW_SIZE.0 as f64,
                WINDOW_SIZE.1 as f64,
            ))
            .with_title("RootVG Prepass Texture Demo");
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

        // --- Custom prepass pipeline -------------------------------------------------------

        // Create any pipeline that renders to a texture. Here we have a basic pipeline
        // that just draws red triangle.

        let prepass_shader = surface
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Shader"),
                source: wgpu::ShaderSource::Wgsl(SHADER.into()),
            });

        let prepass_pipeline_layout =
            surface
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Render Pipeline Layout"),
                    bind_group_layouts: &[],
                    push_constant_ranges: &[],
                });

        let prepass_pipeline =
            surface
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Render Pipeline"),
                    layout: Some(&prepass_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &prepass_shader,
                        entry_point: "vs_main",
                        buffers: &[],
                        compilation_options: PipelineCompilationOptions::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &prepass_shader,
                        entry_point: "fs_main",
                        targets: &[Some(wgpu::ColorTargetState {
                            format: surface.format(),
                            blend: Some(wgpu::BlendState::REPLACE),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: PipelineCompilationOptions::default(),
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        strip_index_format: None,
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: Some(wgpu::Face::Back),
                        polygon_mode: wgpu::PolygonMode::Fill,
                        unclipped_depth: false,
                        conservative: false,
                    },
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState {
                        count: 1,
                        mask: !0,
                        alpha_to_coverage_enabled: false,
                    },
                    multiview: None,
                    cache: None,
                });

        let texture_size = wgpu::Extent3d {
            width: PREPASS_TEXTURE_SIZE.width,
            height: PREPASS_TEXTURE_SIZE.height,
            depth_or_array_layers: 1,
        };
        let prepass_texture = surface.device.create_texture(&wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: surface.format(),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            label: Some("prepass_texture"),
            view_formats: &[],
        });

        // --- Image primitive ---------------------------------------------------------------

        // Create an image primitive that uses the prepass texture as the source.

        let prepass_rc_texture = RcTexture::from_prepass_texture(
            prepass_texture.create_view(&wgpu::TextureViewDescriptor::default()),
            PREPASS_TEXTURE_SIZE,
        );

        let image_primitive = ImagePrimitive::new(prepass_rc_texture, Point::new(180.0, 100.0));

        self.state = Some(State {
            window,
            physical_size,
            scale_factor,
            surface,
            canvas,
            clear_color,

            prepass_pipeline,
            prepass_texture,
            image_primitive,
        });
    }
}

impl ApplicationHandler for PrepassTextureApp {
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
                    let mut cx = state.canvas.begin(state.physical_size, state.scale_factor);

                    // Demonstrate that the texture is indeed being drawn as a
                    // RootVG image by drawing a quad below and above it.

                    cx.add(SolidQuadPrimitive::new(&SolidQuad {
                        bounds: Rect::new(Point::new(100.0, 50.0), Size::new(200.0, 200.0)),
                        bg_color: RGBA8::new(100, 100, 100, 255).into(),
                        ..Default::default()
                    }));

                    cx.set_z_index(1);

                    cx.add(state.image_primitive.clone());

                    cx.set_z_index(2);

                    cx.add(SolidQuadPrimitive::new(&SolidQuad {
                        bounds: Rect::new(Point::new(275.0, 70.0), Size::new(200.0, 200.0)),
                        bg_color: RGBA8::new(200, 100, 200, 100).into(),
                        ..Default::default()
                    }));
                }

                let mut encoder = state
                    .surface
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                // Render the texture in a pre-pass.
                {
                    let view = state
                        .prepass_texture
                        .create_view(&wgpu::TextureViewDescriptor::default());

                    let mut pre_render_pass =
                        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("Render Pass"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color {
                                        r: 0.0,
                                        g: 0.0,
                                        b: 0.0,
                                        a: 0.0,
                                    }),
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            depth_stencil_attachment: None,
                            occlusion_query_set: None,
                            timestamp_writes: None,
                        });

                    pre_render_pass.set_pipeline(&state.prepass_pipeline);
                    pre_render_pass.draw(0..3, 0..1);
                }

                let frame = state.surface.get_current_texture().unwrap();
                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

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
