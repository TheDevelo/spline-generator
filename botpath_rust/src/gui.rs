use egui::Context;
use egui_winit::{EventResponse, State};
use egui_wgpu::renderer::{Renderer, ScreenDescriptor};
use noop_waker::noop_waker;
use rfd::AsyncFileDialog;
use std::future::Future;
use std::pin::Pin;
use winit::event::WindowEvent;
use winit::window::Window;

pub struct Gui {
    // egui variables
    context: Context,
    state: State,
    renderer: Renderer,

    // gui state
    menu_selection: GuiMenu,
    vmf_future: Option<Pin<Box<dyn Future<Output = String>>>>,
}

#[derive(Eq, PartialEq)]
enum GuiMenu {
    Controls,
    Map,
}

pub struct GuiUpdates {
    pub map_vmf: Option<String>,
}

impl std::fmt::Display for GuiMenu {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            GuiMenu::Controls => write!(f, "Controls"),
            GuiMenu::Map => write!(f, "Map"),
        }
    }
}

impl Gui {
    pub fn new(window: &Window, device: &wgpu::Device, texture_format: wgpu::TextureFormat) -> Self {
        let context = egui::Context::default();
        let state = egui_winit::State::new(
            egui::viewport::ViewportId::ROOT,
            window,
            Some(window.scale_factor() as f32),
            None
        );
        let renderer = egui_wgpu::renderer::Renderer::new(
            device,
            texture_format,
            None,
            1
        );

        Gui {
            context,
            state,
            renderer,

            menu_selection: GuiMenu::Controls,
            vmf_future: None,
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> EventResponse {
        self.state.on_window_event(&self.context, event)
    }

    pub fn update(&mut self) -> GuiUpdates {
        let mut updates = GuiUpdates {
            map_vmf: None,
        };

        if let Some(vmf_future) = &mut self.vmf_future {
            // Poll our vmf_future until it is finished loading. This is probably a stupid way to
            // do this, but it would take way more effort to properly do an async setup :)
            let waker = noop_waker();
            let mut ctx = std::task::Context::from_waker(&waker);
            let poll_result = vmf_future.as_mut().poll(&mut ctx);
            if let std::task::Poll::Ready(vmf) = poll_result {
                updates.map_vmf = Some(vmf);
                self.vmf_future = None;
            }
        }

        return updates;
    }

    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        window: &Window,
        total_time: f64,
        screen_desc: ScreenDescriptor
    ) {
        let screen_width = screen_desc.size_in_pixels[0] as f32;
        let screen_height = screen_desc.size_in_pixels[1] as f32;

        // Accumulate input handled by egui
        let mut raw_input = self.state.take_egui_input(window);
        raw_input.time = Some(total_time);

        // Render our egui layout
        let full_ouptut = self.context.run(raw_input, |ctx| {
            let height_pts = screen_height / ctx.pixels_per_point();
            egui::Window::new("Path Controls")
                .anchor(egui::Align2::RIGHT_TOP, (-10.0, 10.0))
                .fixed_size((300.0, height_pts - 55.0))
                .show(&ctx, |ui| {
                    ui.set_width(ui.available_width());
                    ui.set_height(ui.available_height());
                    // Menu selector
                    egui::ComboBox::from_id_source("Menu Selector")
                        .selected_text(format!("{}", self.menu_selection))
                        .show_ui(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.selectable_value(&mut self.menu_selection, GuiMenu::Controls, format!("{}", GuiMenu::Controls));
                            ui.selectable_value(&mut self.menu_selection, GuiMenu::Map, format!("{}", GuiMenu::Map));
                        });
                    ui.separator();

                    match self.menu_selection {
                        GuiMenu::Controls => {
                            ui.label("Controls:");
                            ui.label("WASD: Move around");
                            ui.label("Z: Toggle mouse capture, allowing camera control");
                            ui.label("Mouse: Aim the camera");
                        },
                        GuiMenu::Map => {
                            if ui.button("Load VMF").clicked() {
                                // Spawn a file picker. We'll get the result of the file picker
                                // later in update()
                                self.vmf_future = Some(Box::pin(async {
                                    let map_vmf_file = AsyncFileDialog::new()
                                        .add_filter("VMF", &["vmf"])
                                        .pick_file()
                                        .await;
                                    String::from_utf8(map_vmf_file.unwrap().read().await).unwrap()
                                }));
                            }
                        },
                    }
                });
        });

        // Handle platform functions such as clipboard
        self.state.handle_platform_output(window, &self.context, full_ouptut.platform_output);

        // Prepare egui output for rendering to wgpu
        let tris = self.context.tessellate(full_ouptut.shapes, full_ouptut.pixels_per_point);
        for (id, image_delta) in &full_ouptut.textures_delta.set {
            self.renderer.update_texture(device, queue, *id, &image_delta);
        }
        self.renderer.update_buffers(device, queue, encoder, &tris, &screen_desc);

        // Draw egui to our output view
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            self.renderer.render(&mut render_pass, &tris, &screen_desc);
        }

        // Free any textures requested to be freed
        for free_tex in &full_ouptut.textures_delta.free {
            self.renderer.free_texture(free_tex);
        }
    }
}
