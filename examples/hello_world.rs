use euclid::default::Size2D;
use euclid::rect;
use rgb::RGBA8;
use rootvg::{MeshOpts, Paint};
use std::sync::Arc;
use wgpu::MultisampleState;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

fn main() {
    // Set up logging stuff
    let env = env_logger::Env::default().filter_or("LOG_LEVEL", "warn");
    env_logger::init_from_env(env);

    let event_loop = EventLoop::new().unwrap();
    event_loop.run_app(&mut MyApp::default()).unwrap();
}

#[derive(Default)]
struct MyApp {
    state: Option<State>,
}

impl ApplicationHandler for MyApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_none() {
            let window_attributes = Window::default_attributes()
                .with_inner_size(winit::dpi::LogicalSize::new(800.0, 425.0))
                .with_title("RootVG Hello World Demo");
            let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

            self.state = Some(State::new(window));
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
                    let mut vg = state.vg.begin_frame(state.view_size, state.scale_factor);

                    let mesh = vg
                        .begin_mesh()
                        .rect(rect(30.0, 20.0, 50.0, 60.0))
                        .build(MeshOpts::new().fill(true).stroke_width(4.0).anti_alias(false));

                    vg.paint_fill(mesh, Paint::SolidColor((0.5, 0.4, 0.3, 1.0).into()));
                    //vg.paint_stroke(mesh, Paint::SolidColor((0.6, 0.4, 0.6, 1.0).into()));

                    let mesh = vg
                        .begin_mesh()
                        .rounded_rect(rect(100.0, 200.0, 50.0, 60.0), 6.0)
                        .build(MeshOpts::new().fill(true).stroke_width(4.0));

                    vg.paint_fill(mesh, Paint::SolidColor((0.5, 0.4, 0.6, 1.0).into()));
                    //vg.paint_stroke(mesh, Paint::SolidColor((0.7, 0.4, 0.7, 1.0).into()));
                }

                let frame = state
                    .surface
                    .get_current_texture()
                    .expect("Failed to acquire next swap chain texture");
                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                let mut encoder = state
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                {
                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: None,
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });

                    state.vg.prepare(&state.device, &state.queue);
                    state.vg.render(&mut render_pass);
                }

                state.queue.submit(Some(encoder.finish()));
                frame.present();
            }
            WindowEvent::Resized(new_size) => {
                state.view_size = Size2D::new(new_size.width, new_size.height);
                state.surface_config.width = new_size.width.max(1);
                state.surface_config.height = new_size.height.max(1);
                state
                    .surface
                    .configure(&state.device, &state.surface_config);
                state.window.request_redraw();
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            _ => {}
        }
    }
}

struct State {
    vg: rootvg::Context,

    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    view_size: Size2D<u32>,
    scale_factor: f32,
    window: Arc<Window>,
}

impl State {
    pub fn new(window: Arc<Window>) -> Self {
        pollster::block_on(Self::new_async(window))
    }

    async fn new_async(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let view_size = Size2D::new(size.width, size.height);
        let scale_factor = window.scale_factor() as f32;

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(Arc::clone(&window)).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find an appropriate wgpu adapter");

        println!("{:?}", adapter.get_info());

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                        .using_resolution(adapter.limits()),
                    memory_hints: wgpu::MemoryHints::MemoryUsage,
                },
                None,
            )
            .await
            .expect("Failed to create wgpu device");

        let surface_config = surface
            .get_default_config(&adapter, size.width, size.height)
            .unwrap();
        surface.configure(&device, &surface_config);

        println!("{:?}", &surface_config);

        let texture_format = surface_config.format;

        let vg = rootvg::Context::new(
            &device,
            texture_format,
            MultisampleState::default(),
            scale_factor,
            true,
        );

        Self {
            vg,
            surface,
            device,
            queue,
            surface_config,
            window,
            view_size,
            scale_factor,
        }
    }
}
