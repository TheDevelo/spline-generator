use crate::RenderState;
use crate::world::{map, spline, World};
use crate::world::spline::export;

use egui::{Color32, Context, DragValue};
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
    state: State,
    renderer: Renderer,

    // gui state
    menu_selection: GuiMenu,
    snap_value: f32,

    vmf_future: Option<Pin<Box<dyn Future<Output = Option<String>>>>>,
    load_state_future: Option<Pin<Box<dyn Future<Output = Option<String>>>>>,
    save_state_future: Option<Pin<Box<dyn Future<Output = ()>>>>,
    export_spline_future: Option<Pin<Box<dyn Future<Output = ()>>>>,

    avg_frame_time: f64,
    window_swapped: bool,
}

#[derive(Eq, PartialEq)]
enum GuiMenu {
    Controls,
    Map,
    Spline,
}

impl std::fmt::Display for GuiMenu {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            GuiMenu::Controls => write!(f, "Controls"),
            GuiMenu::Map => write!(f, "Map"),
            GuiMenu::Spline => write!(f, "Spline"),
        }
    }
}

impl Gui {
    pub fn new(render_state: &RenderState) -> Self {
        let context = Context::default();
        let state = State::new(
            context,
            egui::viewport::ViewportId::ROOT,
            &render_state.window,
            Some(render_state.window.scale_factor() as f32),
            None
        );
        let renderer = Renderer::new(
            &render_state.device,
            render_state.config.format,
            None,
            1
        );

        Gui {
            state,
            renderer,

            menu_selection: GuiMenu::Controls,
            snap_value: 64.0,

            vmf_future: None,
            load_state_future: None,
            save_state_future: None,
            export_spline_future: None,

            avg_frame_time: 1.0 / 60.0, // 60 FPS is a reasonable starting assumption
            window_swapped: false,
        }
    }

    pub fn input(&mut self, window: &Window, event: &WindowEvent) -> EventResponse {
        self.state.on_window_event(window, event)
    }

    pub fn update(&mut self, render_state: &RenderState, world: &mut World) {
        if let Some(vmf_future) = &mut self.vmf_future {
            // Poll our vmf_future until it is finished loading. This is probably a stupid way to
            // do this, but it would take way more effort to properly do an async setup :)
            let waker = noop_waker();
            let mut ctx = std::task::Context::from_waker(&waker);
            let poll_result = vmf_future.as_mut().poll(&mut ctx);
            if let std::task::Poll::Ready(vmf) = poll_result {
                // vmf_future is ready, so update map
                // check if we managed to actually load a vmf file first
                if let Some(vmf) = vmf {
                    world.map = map::Map::from_string(&vmf, &render_state.device).unwrap();
                }
                self.vmf_future = None;
            }
        }

        if let Some(load_state_future) = &mut self.load_state_future {
            // Same polling setup as above
            let waker = noop_waker();
            let mut ctx = std::task::Context::from_waker(&waker);
            let poll_result = load_state_future.as_mut().poll(&mut ctx);
            if let std::task::Poll::Ready(save) = poll_result {
                // Load the spline state into the interface
                if let Some(save) = save {
                    world.restore_state(&save, render_state);
                }
                self.load_state_future = None;
            }
        }

        if let Some(save_state_future) = &mut self.save_state_future {
            // Same polling setup as above, but we just set to none if finished
            let waker = noop_waker();
            let mut ctx = std::task::Context::from_waker(&waker);
            let poll_result = save_state_future.as_mut().poll(&mut ctx);
            if let std::task::Poll::Ready(_) = poll_result {
                self.save_state_future = None;
            }
        }

        if let Some(export_spline_future) = &mut self.export_spline_future {
            // Same polling setup as above, but we just set to none if finished
            let waker = noop_waker();
            let mut ctx = std::task::Context::from_waker(&waker);
            let poll_result = export_spline_future.as_mut().poll(&mut ctx);
            if let std::task::Poll::Ready(_) = poll_result {
                self.export_spline_future = None;
            }
        }
    }

    pub fn render(&mut self, render_state: &RenderState, world: &mut World, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView, dt: f64, total_time: f64) {
        // Update our FPS average. In order to get a rolling average without storing the frame
        // times for the past X frames, we compute it using a geometric series sum.
        self.avg_frame_time = 15.0 * self.avg_frame_time / 16.0 + dt / 16.0;

        // Accumulate input handled by egui
        let mut raw_input = self.state.take_egui_input(&render_state.window);
        raw_input.time = Some(total_time);

        // Render our egui layout
        let full_ouptut = self.state.egui_ctx().run(raw_input, |ctx| {
            let height_pts = render_state.size.height as f32 / ctx.pixels_per_point();

            let main_anchor;
            let fps_anchor;
            let main_offset;
            let fps_offset;
            if self.window_swapped {
                main_anchor = egui::Align2::LEFT_TOP;
                fps_anchor = egui::Align2::RIGHT_TOP;
                main_offset = (10.0, 10.0);
                fps_offset = (-10.0, 10.0);
            }
            else {
                main_anchor = egui::Align2::RIGHT_TOP;
                fps_anchor = egui::Align2::LEFT_TOP;
                main_offset = (-10.0, 10.0);
                fps_offset = (10.0, 10.0);
            }

            egui::Window::new("Path Controls")
                .anchor(main_anchor, main_offset)
                .fixed_size((300.0, height_pts - 55.0))
                .show(&ctx, |ui| {
                    ui.set_width(ui.available_width());
                    ui.set_height(ui.available_height());
                    ui.add_space(8.0);
                    // Menu selector
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_source("Menu Selector")
                            .selected_text(format!("{}", self.menu_selection))
                            .show_ui(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.selectable_value(&mut self.menu_selection, GuiMenu::Controls, format!("{}", GuiMenu::Controls));
                                ui.selectable_value(&mut self.menu_selection, GuiMenu::Map, format!("{}", GuiMenu::Map));
                                ui.selectable_value(&mut self.menu_selection, GuiMenu::Spline, format!("{}", GuiMenu::Spline));
                            });
                        ui.allocate_space(ui.available_size());
                        if ui.button("Swap Sides").clicked() {
                            self.window_swapped = !self.window_swapped;
                        }
                    });
                    ui.separator();

                    match self.menu_selection {
                        GuiMenu::Controls => {
                            ui.label("Controls:");
                            ui.label("WASD: Move around");
                            ui.label("Shift: Speed up movement");
                            ui.label("Z: Toggle mouse capture, allowing camera control");
                            ui.label("Mouse: Aim the camera");
                            ui.label("Space: Insert a new point into the current spline");
                            ui.label("Left & Right Arrow Keys: Change the selected point on the current spline");
                        },
                        GuiMenu::Map => {
                            if ui.button("Load VMF").clicked() && self.vmf_future.is_none() {
                                // Spawn a file picker. We'll get the result of the file picker
                                // later in update()
                                self.vmf_future = Some(Box::pin(async {
                                    let map_vmf_file = AsyncFileDialog::new()
                                        .add_filter("VMF", &["vmf"])
                                        .pick_file()
                                        .await;
                                    if let Some(map_vmf_file) = map_vmf_file {
                                        String::from_utf8(map_vmf_file.read().await).ok()
                                    }
                                    else {
                                        None
                                    }
                                }));
                            }
                            ui.separator();
                            ui.horizontal(|ui| {
                                if ui.button("Load splines").clicked() && self.load_state_future.is_none() {
                                    // Spawn a file picker for the spline save file
                                    self.load_state_future = Some(Box::pin(async {
                                        let save_file = AsyncFileDialog::new()
                                            .add_filter("Spline state (.json)", &["json"])
                                            .pick_file()
                                            .await;
                                        if let Some(save_file) = save_file {
                                            String::from_utf8(save_file.read().await).ok()
                                        }
                                        else {
                                            None
                                        }
                                    }));
                                }
                                if ui.button("Save splines").clicked() {
                                    // Serialize our state, spawn a file picker, and write to the
                                    // selected file
                                    let serialized_state = world.save_state();
                                    self.save_state_future = Some(Box::pin(async {
                                        let save_file = AsyncFileDialog::new()
                                            .add_filter("Spline state (.json)", &["json"])
                                            .set_file_name("splines.json")
                                            .save_file()
                                            .await;
                                        if let Some(save_handle) = save_file {
                                            let _ = save_handle.write(&serialized_state.into_bytes()).await;
                                        };
                                    }));
                                }
                            });
                        },
                        GuiMenu::Spline => {
                            ui.horizontal(|ui| {
                                if ui.button("+").clicked() {
                                    world.add_spline();
                                }
                                if ui.button("-").clicked() && world.splines.len() > 0 {
                                    world.splines.remove(world.selected_spline as usize);
                                    if world.selected_spline != 0 {
                                        world.selected_spline -= 1;
                                    }
                                }

                                let selected_spline_text;
                                if world.splines.len() > 0 {
                                    selected_spline_text = format!("Spline {} - {}", world.selected_spline + 1, world.splines[world.selected_spline as usize].data.name);
                                }
                                else {
                                    selected_spline_text = "".to_string();
                                }
                                egui::ComboBox::from_id_source("Spline Selector")
                                    .selected_text(selected_spline_text)
                                    .show_ui(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        for (i, spline) in world.splines.iter().enumerate() {
                                            ui.selectable_value(&mut world.selected_spline, i as u32, format!("Spline {} - {}", i + 1, spline.data.name));
                                        }
                                    });
                            });
                            if ui.button("Export").clicked() {
                                // Write out a zip file containing the uncompiled spline model
                                let zip_bytes = export::construct_zip(&world.splines).unwrap();
                                self.export_spline_future = Some(Box::pin(async {
                                    let zip_bytes = zip_bytes; // Need this to move zip_bytes inside the closure
                                    let save_file = AsyncFileDialog::new()
                                        .add_filter("Export archive (.zip)", &["zip"])
                                        .set_file_name("model_export.zip")
                                        .save_file()
                                        .await;
                                    if let Some(save_handle) = save_file {
                                        let _ = save_handle.write(&zip_bytes).await;
                                    };
                                }));
                            }
                            ui.separator();

                            if world.splines.len() > 0 {
                                let spline = &mut world.splines[world.selected_spline as usize];
                                let mut rebuild_spline = false;
                                ui.label("Spline properties");
                                ui.horizontal(|ui| {
                                    ui.label("Radius:");
                                    if ui.add(DragValue::new(&mut spline.data.radius)).changed() {
                                        rebuild_spline = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Sides:");
                                    if ui.add(DragValue::new(&mut spline.data.sides).clamp_range(1..=std::u32::MAX)).changed() {
                                        rebuild_spline = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Subdivisions:");
                                    if ui.add(DragValue::new(&mut spline.data.subdivisions).clamp_range(1..=std::u32::MAX)).changed() {
                                        rebuild_spline = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Model Path:");
                                    ui.text_edit_singleline(&mut spline.data.name);
                                });
                                ui.separator();

                                let enabled = spline.selected_point < spline.data.points.len() as u32;
                                // Default values to show in case we don't have a selected point and the UI is disabled
                                let mut default_point = spline::SplineControlPoint {
                                    position: cgmath::Point3::new(0.0, 0.0, 0.0),
                                    pitch: cgmath::Deg(0.0),
                                    yaw: cgmath::Deg(0.0),
                                    tangent_magnitude: 0.0,
                                    color: Color32::WHITE,
                                };

                                if enabled {
                                    ui.label(format!("Point properties - Selected point: {}", spline.selected_point + 1));
                                }
                                else {
                                    ui.label("Point properties");
                                }
                                ui.add_enabled_ui(enabled, |ui| {
                                    ui.horizontal(|ui| {
                                        if ui.button("+").clicked() {
                                            spline.add_before_selected();
                                        }
                                        if ui.button("-").clicked() {
                                            spline.data.points.remove(spline.selected_point as usize);
                                            rebuild_spline = true;
                                        }
                                    });

                                    let point = spline.data.points.get_mut(spline.selected_point as usize).unwrap_or(&mut default_point);
                                    ui.horizontal(|ui| {
                                        ui.label("X:");
                                        if ui.add(DragValue::new(&mut point.position.x)).changed() {
                                            rebuild_spline = true;
                                        }
                                        ui.label("Y:");
                                        if ui.add(DragValue::new(&mut point.position.y)).changed() {
                                            rebuild_spline = true;
                                        }
                                        ui.label("Z:");
                                        if ui.add(DragValue::new(&mut point.position.z)).changed() {
                                            rebuild_spline = true;
                                        }
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Snap position to");
                                        ui.add(DragValue::new(&mut self.snap_value));
                                        ui.label("-");
                                        if ui.button("Snap").clicked() {
                                            point.position.x = (point.position.x / self.snap_value).round() * self.snap_value;
                                            point.position.y = (point.position.y / self.snap_value).round() * self.snap_value;
                                            point.position.z = (point.position.z / self.snap_value).round() * self.snap_value;
                                            rebuild_spline = true;
                                        }
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Pitch:");
                                        if ui.add(DragValue::new(&mut point.pitch.0)).changed() {
                                            rebuild_spline = true;
                                        }
                                        ui.label("Yaw:");
                                        if ui.add(DragValue::new(&mut point.yaw.0)).changed() {
                                            rebuild_spline = true;
                                        }
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Tangent Magnitude:");
                                        if ui.add(DragValue::new(&mut point.tangent_magnitude)).changed() {
                                            rebuild_spline = true;
                                        }
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Color:");
                                        ui.color_edit_button_srgba(&mut point.color);
                                    });
                                });

                                if rebuild_spline {
                                    spline.request_rebuild();
                                }
                            }
                        }
                    }
                });

            egui::Window::new("FPS Counter")
                .anchor(fps_anchor, fps_offset)
                .resizable(false)
                .title_bar(false)
                .frame(egui::Frame {
                    fill: egui::Color32::TRANSPARENT,
                    ..Default::default()
                }).show(&ctx, |ui| {
                    ui.colored_label(egui::Color32::BLACK, format!("{:.1} FPS", 1.0 / self.avg_frame_time));
                });
        });

        // Handle platform functions such as clipboard
        self.state.handle_platform_output(&render_state.window, full_ouptut.platform_output);

        // Prepare egui output for rendering to wgpu
        let screen_desc = ScreenDescriptor {
            size_in_pixels: render_state.size.into(),
            pixels_per_point: render_state.window.scale_factor() as f32,
        };
        let tris = self.state.egui_ctx().tessellate(full_ouptut.shapes, full_ouptut.pixels_per_point);
        for (id, image_delta) in &full_ouptut.textures_delta.set {
            self.renderer.update_texture(&render_state.device, &render_state.queue, *id, &image_delta);
        }
        self.renderer.update_buffers(&render_state.device, &render_state.queue, encoder, &tris, &screen_desc);

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
