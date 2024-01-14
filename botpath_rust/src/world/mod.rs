mod camera;
pub mod spline;
pub mod map;

use crate::texture;
use crate::RenderState;

use web_time::Duration;
use winit::event::*;
use wgpu::util::DeviceExt;

// We make some fields pub so that the GUI can inspect/modify them
pub struct World {
    depth_texture: texture::Texture,
    camera: camera::Camera,
    camera_uniform: camera::CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera_controller: camera::CameraController,
    map_renderer: map::MapRenderer,
    pub map: map::Map,
    spline_renderer: spline::SplineRenderer,
    pub spline: spline::Spline,
}

impl World {
    pub fn new(render_state: &RenderState) -> Self {
        let depth_texture = texture::Texture::create_depth_texture(&render_state.device, &render_state.config, "depth_texture");

        let camera = camera::Camera {
            position: (0.0, 0.0, 0.0).into(),
            pitch: cgmath::Rad(0.0),
            yaw: cgmath::Rad(0.0),
            aspect: render_state.config.width as f32 / render_state.config.height as f32,
            fovy: 60.0,
            znear: 1.0,
            zfar: 10000.0,
        };

        let mut camera_uniform = camera::CameraUniform::new();
        camera_uniform.update_view_proj(&camera);

        let camera_buffer = render_state.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout = render_state.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("camera_bind_group_layout"),
        });

        let camera_bind_group = render_state.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                }
            ],
            label: Some("camera_bind_group"),
        });

        let camera_controller = camera::CameraController::new(2500.0, std::f32::consts::PI / 1000.0);

        let map_renderer = map::MapRenderer::new(render_state, &camera_bind_group_layout);
        let map = map::Map::empty(&render_state.device);
        let spline_renderer = spline::SplineRenderer::new(render_state, &camera_bind_group_layout);
        let spline = spline::Spline::new(&render_state.device, &spline_renderer);

        Self {
            depth_texture,
            camera,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            camera_controller,
            map_renderer,
            map,
            spline_renderer,
            spline,
        }
    }

    pub fn resize(&mut self, render_state: &RenderState) {
        self.camera.aspect = render_state.size.width as f32 / render_state.size.height as f32;
        self.depth_texture = texture::Texture::create_depth_texture(&render_state.device, &render_state.config, "depth_texture");
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        // Camera controller events
        if self.camera_controller.process_events(event) {
            return true;
        }

        // Spline control events
        if self.spline.process_events(event, &self.camera) {
            return true;
        }

        return false;
    }

    pub fn process_mouse(&mut self, delta: (f64, f64)) {
        self.camera_controller.process_mouse(delta);
    }

    pub fn update(&mut self, render_state: &RenderState, dt: Duration) {
        self.camera_controller.update_camera(&mut self.camera, dt);
        self.camera_uniform.update_view_proj(&self.camera);
        render_state.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera_uniform]));

        self.spline.update(render_state);
    }

    pub fn render(&self, _render_state: &RenderState, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("3D Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        self.map_renderer.draw(&mut render_pass, &self.camera_bind_group, &self.map);
        self.spline_renderer.draw(&mut render_pass, &self.camera_bind_group, &self.spline);
    }
}
