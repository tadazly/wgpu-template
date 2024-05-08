use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

#[derive(Default)]
struct App {
    window: Option<Arc<Window>>,
    state: Option<State>,
    scale_factor: f64,
}

impl App {
    fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                let state = self.state.as_mut().unwrap();
                state.background_color.r = position.x / state.size.width as f64;
                state.background_color.g = position.y / state.size.height as f64;
                state.background_color.b = (position.x + position.y) / (state.size.height + state.size.height) as f64;
                self.window.as_ref().unwrap().request_redraw();
                true
            }
            _ => false
        }
    }

    fn exit(&mut self, event_loop: &ActiveEventLoop) {
        println!("Exit App !");

        // https://github.com/rust-windowing/winit/issues/3668
        // Use Option::take to Dispose Option<Arc<Window>>
        self.window.take();
        self.state.take();
        event_loop.exit();
    }
}

struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    background_color: wgpu::Color,
}

impl State {
    async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .filter(|f| f.is_srgb())
            .next()
            .unwrap_or(surface_caps.formats[0]);

        let modes = &surface_caps.present_modes;
        println!("Available present_modes: {modes:?}");

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let background_color = wgpu::Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        };

        Self {
            surface,
            device,
            queue,
            config,
            size,
            background_color,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn update(&mut self) {
        // todo!()
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // The `encoder` builds a command buffer that we can then send to the GPU.
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(self.background_color),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        // begin_render_pass() borrows encoder mutably (aka &mut self). We can't call encoder.finish() until we release that mutable borrow. 
        drop(render_pass);

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        println!("App Resumed !");

        // Initialized at first Resumed Event
        if self.window.is_none() {
            println!("Create Window !");
            let window_attributes = Window::default_attributes()
                .with_title("wgpu-template")
                .with_inner_size(winit::dpi::LogicalSize::new(640.0, 480.0));

            let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
            let state = pollster::block_on(State::new(Arc::clone(&window)));
            println!("Bind Window !");

            self.scale_factor = window.scale_factor();
            self.window = Some(window);
            self.state = Some(state);
            self.window.as_ref().unwrap().pre_present_notify();
            self.window.as_ref().unwrap().request_redraw();
        }
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        println!("App Suspended !");
    }

    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: winit::event::StartCause) {
        // println!("new_events: {cause:?}");
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if self.input(&event) {
            return;
        }
        // println!("WindowEvent: {event:?}");
        match event {
            WindowEvent::CloseRequested => self.exit(event_loop),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key,
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => match physical_key {
                PhysicalKey::Code(KeyCode::Escape) => self.exit(event_loop),
                _ => (),
            },
            WindowEvent::RedrawRequested => {
                let state = self.state.as_mut().unwrap();
                state.update();
                match state.render() {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("{:?}", e);
                        match e {
                            wgpu::SurfaceError::Lost => state.resize(state.size),
                            wgpu::SurfaceError::OutOfMemory => self.exit(event_loop),
                            _ => ()
                        }
                    }
                }
            }
            WindowEvent::Resized(physical_size) => {
                println!("On Resized !");
                self.state.as_mut().unwrap().resize(physical_size);
            }
            WindowEvent::ScaleFactorChanged {
                scale_factor,
                mut inner_size_writer,
            } => {
                let PhysicalSize { width, height } = self.state.as_ref().unwrap().size;
                let new_width = width as f64 / self.scale_factor * scale_factor;
                let new_height = height as f64 / self.scale_factor * scale_factor;
                let new_inner_size =
                    PhysicalSize::new(new_width.floor() as u32, new_height.floor() as u32);
                let _ = inner_size_writer.request_inner_size(new_inner_size);
                println!("Request new size: {new_inner_size:?}");
            }
            _ => (),
        }
    }
}

pub fn run() -> Result<(), winit::error::EventLoopError> {
    let mut app = App::default();
    let event_loop = EventLoop::new().unwrap();

    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut app)
}
