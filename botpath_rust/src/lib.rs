mod gui;
mod texture;
mod world;

use web_time::{Duration, Instant};
use winit::{
    event::*,
    event_loop::EventLoop,
    keyboard::Key,
    window::{CursorGrabMode, WindowBuilder, Window},
};
#[cfg(target_arch="wasm32")]
use wasm_bindgen::prelude::*;

pub trait Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static>;
}

struct State {
    render_state: RenderState,
    world: world::World,
    gui: gui::Gui,
    camera_lock: bool,
}

struct RenderState {
    window: Window,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
}

impl State {
    async fn new(window: Window) -> Self {
        let mut size = window.inner_size();
        // Our window starts out with a size of 0x0 on WASM, so we need to give our window an actual
        // size when we initialize. The actual size will be provided later by resize()
        // 4x4 is the minimum possible size, so set either dimension to 4.
        if size.width == 0 {
            size.width = 4;
        }
        if size.height == 0 {
            size.height = 4;
        }

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            }
        ).await.unwrap();

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                // WebGL doesn't support all features of WGPU, so downlevel on web
                limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                }
                else {
                    wgpu::Limits::default()
                },
                label: None,
            },
            None
        ).await.unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        // Use sRGB for our color format on surface textures
        let surface_format = surface_caps.formats.iter()
            .copied()
            .filter(|f| f.is_srgb())
            .next()
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };

        // Wait to configure surface
        surface.configure(&device, &config);

        let render_state = RenderState {
            window,
            surface,
            device,
            queue,
            config,
            size,
        };

        let world = world::World::new(&render_state);
        let gui = gui::Gui::new(&render_state);

        Self {
            render_state,
            world,
            gui,
            camera_lock: false,
        }
    }

    pub fn window(&self) -> &Window {
        &self.render_state.window
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.render_state.size = new_size;
            self.render_state.config.width = new_size.width;
            self.render_state.config.height = new_size.height;
            self.render_state.surface.configure(&self.render_state.device, &self.render_state.config);

            self.world.resize(&self.render_state);
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        // Start by letting egui handle any inputs first. Then if the input wasn't consumed, we can
        // handle it ourselves. The one edge case is when camera lock is enabled. We don't want our
        // mouse/keyboard to affect egui, so don't forward if camera lock is enabled.
        // This might cause issues if some non-input events happens (such as window resize), but
        // those events shouldn't happen during camera lock.
        let mut response = egui_winit::EventResponse { consumed: false, repaint: false };
        if !self.camera_lock {
            response = self.gui.input(&self.render_state.window, event);
        }
        if response.repaint {
            self.render_state.window.request_redraw();
        }
        if response.consumed {
            return true;
        }

        // World events
        if self.world.input(event) {
            return true;
        }

        // Camera lock related events
        match event {
            WindowEvent::KeyboardInput {
                event: KeyEvent {
                    state,
                    logical_key,
                    ..
                },
                ..
            } => {
                if *state == ElementState::Pressed {
                    match logical_key.as_ref() {
                        Key::Character("z") | Key::Character("Z") => {
                            self.camera_lock = !self.camera_lock;
                            if self.camera_lock {
                                // Lock and hide the mouse. Since winit (at least currently)
                                // doesn't support locked mode on all relevant platforms, use
                                // confined mode as a fallback.
                                if self.render_state.window.set_cursor_grab(CursorGrabMode::Locked).is_err() {
                                    if self.render_state.window.set_cursor_grab(CursorGrabMode::Confined).is_err() {
                                        eprintln!("failed to properly set the cursor grab mode!");
                                    }
                                }
                                self.render_state.window.set_cursor_visible(false);
                            }
                            else {
                                if self.render_state.window.set_cursor_grab(CursorGrabMode::None).is_err() {
                                    eprintln!("failed to properly unset the cursor grab mode!");
                                }
                                self.render_state.window.set_cursor_visible(true);
                            }
                            return true;
                        }
                        _ => {}
                    }
                }
            },
            WindowEvent::Focused(focused) if *focused == false => {
                self.camera_lock = false;
                if self.render_state.window.set_cursor_grab(CursorGrabMode::None).is_err() {
                    eprintln!("failed to properly unset the cursor grab mode!");
                }
                self.render_state.window.set_cursor_visible(true);
            },
            _ => {}
        }

        return false;
    }

    fn input_mouse_delta(&mut self, delta: (f64, f64)) {
        // Only send mouse input to the camera controller if we have locked the mouse
        if self.camera_lock {
            self.world.process_mouse(delta);
        }
    }

    fn update(&mut self, dt: Duration) {
        // Process updates from the world
        self.world.update(&self.render_state, dt);

        // Process updates from the GUI
        self.gui.update(&self.render_state, &mut self.world);
    }

    fn render(&mut self, dt: Duration, total_time: Duration) -> Result<(), wgpu::SurfaceError> {
        let output = self.render_state.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.render_state.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        // World Render Pass
        self.world.render(&self.render_state, &mut encoder, &view);

        // egui Render Pass
        self.gui.render(&self.render_state, &mut encoder, &view, dt.as_secs_f64(), total_time.as_secs_f64());

        // submit will accept anything that implements IntoIter
        self.render_state.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

#[cfg_attr(target_arch="wasm32", wasm_bindgen(start))]
pub async fn run() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Warn).expect("Couldn't initialize logger");
        } else {
            env_logger::init();
        }
    }

    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| {
                let dst = doc.get_element_by_id("wgpu-target")?;
                let canvas = web_sys::Element::from(window.canvas().unwrap());
                dst.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to document body.");
    }

    let mut state = State::new(window).await;
    let mut total_time = Duration::ZERO;
    let mut last_render_time = Instant::now();

    event_loop.run(move |event, elwt| {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.window().id() => if !state.input(event) {
                match event {
                    WindowEvent::CloseRequested => elwt.exit(),
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);
                    },
                    WindowEvent::RedrawRequested => {
                        let now = Instant::now();
                        let dt = now - last_render_time;
                        total_time += dt;
                        last_render_time = now;
                        state.update(dt);

                        match state.render(dt, total_time) {
                            Ok(_) => {}
                            // Reconfigure the surface if lost
                            Err(wgpu::SurfaceError::Lost) => state.resize(state.render_state.size),
                            // The system is out of memory, we should probably quit
                            Err(wgpu::SurfaceError::OutOfMemory) => elwt.exit(),
                            // All other errors (Outdated, Timeout) should be resolved by the next frame
                            Err(e) => eprintln!("{:?}", e),
                        }

                    },
                    _ => {}
                }
            },
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta },
                ..
            } => {
                state.input_mouse_delta(delta);
                state.window().request_redraw();
            },
            Event::AboutToWait => {
                // RedrawRequested will only trigger once unless we manually request it.
                state.window().request_redraw();
            }
            _ => {}
        }
    }).unwrap();
}
