/// An example custom primitive that renders triangle with a solid color.
mod my_custom_primitive {
    use bytemuck::{Pod, Zeroable};
    use rootvg::{
        buffer::Buffer,
        color::{PackedSrgb, RGBA8},
        math::{Point, Size},
        pipeline::{CustomPipeline, CustomPipelinePrimitive, DefaultConstantUniforms},
    };
    use wgpu::PipelineCompilationOptions;

    const INITIAL_INSTANCES: usize = 8;

    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
    pub struct MyCustomPrimitive {
        pub color: PackedSrgb,
        pub position: [f32; 2],
        pub size: [f32; 2],
    }

    impl MyCustomPrimitive {
        pub fn new(color: RGBA8, position: Point, size: Size) -> Self {
            Self {
                color: color.into(),
                position: position.into(),
                size: size.into(),
            }
        }
    }

    pub struct MyCustomPrimitivePipeline {
        pipeline: wgpu::RenderPipeline,

        constants_buffer: wgpu::Buffer,
        constants_bind_group: wgpu::BindGroup,

        vertex_buffer: Buffer<MyCustomPrimitive>,
        num_vertices: usize,
    }

    impl MyCustomPrimitivePipeline {
        pub fn new(
            device: &wgpu::Device,
            format: wgpu::TextureFormat,
            multisample: wgpu::MultisampleState,
        ) -> Self {
            let (constants_layout, constants_buffer, constants_bind_group) =
                DefaultConstantUniforms::layout_buffer_and_bind_group(device);

            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("rootvg-quad solid pipeline layout"),
                push_constant_ranges: &[],
                bind_group_layouts: &[&constants_layout],
            });

            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("my custom primitive shader"),
                source: wgpu::ShaderSource::Wgsl(SHADER.into()),
            });

            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("my custom primitive pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<MyCustomPrimitive>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array!(
                            // Color
                            0 => Float32x4,
                            // Position
                            1 => Float32x2,
                            // Size
                            2 => Float32x2,
                        ),
                    }],
                    compilation_options: PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::SrcAlpha,
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                operation: wgpu::BlendOperation::Add,
                            },
                            alpha: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::One,
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                operation: wgpu::BlendOperation::Add,
                            },
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    front_face: wgpu::FrontFace::Cw,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample,
                multiview: None,
                cache: None,
            });

            let vertex_buffer = Buffer::new(
                device,
                "my custom primitive vertex buffer",
                INITIAL_INSTANCES,
                wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            );

            Self {
                pipeline,
                constants_buffer,
                constants_bind_group,
                vertex_buffer,
                num_vertices: 0,
            }
        }
    }

    impl CustomPipeline for MyCustomPrimitivePipeline {
        fn prepare(
            &mut self,
            device: &wgpu::Device,
            queue: &wgpu::Queue,
            screen_size: rootvg::math::PhysicalSizeI32,
            scale_factor: rootvg::math::ScaleFactor,
            primitives: &[CustomPipelinePrimitive],
        ) -> Result<(), Box<dyn std::error::Error>> {
            DefaultConstantUniforms::prepare_buffer(
                &self.constants_buffer,
                screen_size,
                scale_factor,
                queue,
            );

            let vertices: Vec<MyCustomPrimitive> = primitives
                .iter()
                .map(|p| {
                    let mut primitive = *p.primitive.downcast_ref::<MyCustomPrimitive>().unwrap();

                    // Offset the primitive by the requested amount
                    primitive.position[0] += p.offset.x;
                    primitive.position[1] += p.offset.y;

                    primitive
                })
                .collect();

            self.vertex_buffer
                .expand_to_fit_new_size(device, primitives.len());
            self.vertex_buffer.write(queue, 0, &vertices);

            self.num_vertices = vertices.len();

            Ok(())
        }

        fn render_primitive<'pass>(
            &'pass self,
            primitive_index: usize,
            render_pass: &mut wgpu::RenderPass<'pass>,
        ) -> Result<(), Box<dyn std::error::Error>> {
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.constants_bind_group, &[]);

            render_pass.set_vertex_buffer(
                0,
                self.vertex_buffer
                    .slice(primitive_index..primitive_index + 1),
            );
            render_pass.draw(0..3, 0..1);

            Ok(())
        }
    }

    static SHADER: &'static str = "
struct Globals {
    screen_size_recip: vec2f,
    scale_factor: f32,
}

@group(0) @binding(0) var<uniform> globals: Globals;

struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) color: vec4f,
    @location(1) pos: vec2f,
    @location(2) size: vec2f,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) color: vec4f,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let x = (f32(1 - i32(input.vertex_index)) + 1.0) / 2.0;
    let y = (f32(i32(input.vertex_index & 1u) * 2 - 1) + 1.0) / 2.0;

    let screen_pos: vec2f = (input.pos + (vec2f(x, y) * input.size)) * globals.scale_factor;
    out.clip_position = vec4<f32>(
        (screen_pos.x * globals.screen_size_recip.x) - 1.0,
        1.0 - (screen_pos.y * globals.screen_size_recip.y),
        0.0,
        1.0
    );

    out.color = input.color;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    return in.color;
}
";
}

// ---------------------------------------------------------------------------------------

use rootvg::math::{PhysicalSizeI32, Point, Rect, ScaleFactor, Size, Vector};
use rootvg::quad::{SolidQuad, SolidQuadPrimitive};
use rootvg::{color::RGBA8, pipeline::CustomPrimitive, surface::DefaultSurface, Canvas};
use rootvg_text::{glyphon::FontSystem, svg::SvgIconSystem};
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

use self::my_custom_primitive::{MyCustomPrimitive, MyCustomPrimitivePipeline};

const WINDOW_SIZE: (f32, f32) = (800.0, 425.0);

fn main() {
    // Set up logging stuff
    let env = env_logger::Env::default().filter_or("LOG_LEVEL", "info");
    env_logger::init_from_env(env);

    let event_loop = EventLoop::new().unwrap();
    event_loop
        .run_app(&mut CustomPrimitiveApp {
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

    my_custom_primitive_1: CustomPrimitive,
    my_custom_primitive_2: CustomPrimitive,
}

struct CustomPrimitiveApp {
    state: Option<State>,
    font_system: FontSystem,
    svg_icon_system: SvgIconSystem,
}

impl CustomPrimitiveApp {
    fn create_window(&mut self, event_loop: &ActiveEventLoop) {
        // --- Set up winit window -----------------------------------------------------------

        let window_attributes = Window::default_attributes()
            .with_inner_size(winit::dpi::LogicalSize::new(
                WINDOW_SIZE.0 as f64,
                WINDOW_SIZE.1 as f64,
            ))
            .with_title("RootVG Custom Primitive Demo");
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

        let canvas_config = surface.canvas_config();

        // --- Canvas ------------------------------------------------------------------------

        // A `Canvas` automatically batches primitives and renders them to a
        // render target (such as the output framebuffer).
        let mut canvas = rootvg::Canvas::new(
            &surface.device,
            &surface.queue,
            surface.format(),
            canvas_config,
            &mut self.font_system,
        );

        // --- Initialize custom pipeline ----------------------------------------------------

        let my_custom_pipeline_id = canvas.insert_custom_pipeline(MyCustomPrimitivePipeline::new(
            &surface.device,
            surface.format(),
            canvas_config.multisample,
        ));

        // --- Create custom primitives ------------------------------------------------------

        let my_custom_primitive_1 = CustomPrimitive::new(
            MyCustomPrimitive::new(
                RGBA8::new(255, 0, 0, 255),
                Point::new(110.0, 100.0),
                Size::new(100.0, 100.0),
            ),
            my_custom_pipeline_id,
        );

        let my_custom_primitive_2 = CustomPrimitive::new_with_offset(
            MyCustomPrimitive::new(
                RGBA8::new(0, 255, 255, 255),
                Point::new(100.0, 100.0),
                Size::new(100.0, 100.0),
            ),
            Vector::new(100.0, 100.0),
            my_custom_pipeline_id,
        );

        self.state = Some(State {
            window,
            physical_size,
            scale_factor,
            surface,
            canvas,
            my_custom_primitive_1,
            my_custom_primitive_2,
        });
    }
}

impl ApplicationHandler for CustomPrimitiveApp {
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

                    // Demonstrate that the custom primitives are indeed being drawn
                    // as part of the canvas's render pass by drawing a quad below and
                    // above it.

                    cx.add(SolidQuadPrimitive::new(&SolidQuad {
                        bounds: Rect::new(Point::new(100.0, 50.0), Size::new(200.0, 200.0)),
                        bg_color: RGBA8::new(100, 100, 100, 255).into(),
                        ..Default::default()
                    }));

                    cx.set_z_index(1);

                    cx.add(state.my_custom_primitive_1.clone());
                    cx.add(state.my_custom_primitive_2.clone());

                    cx.set_z_index(2);

                    cx.add(SolidQuadPrimitive::new(&SolidQuad {
                        bounds: Rect::new(Point::new(275.0, 70.0), Size::new(200.0, 200.0)),
                        bg_color: RGBA8::new(200, 100, 200, 100).into(),
                        ..Default::default()
                    }));
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
                        Some(RGBA8::new(0, 0, 0, 255).into()),
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
