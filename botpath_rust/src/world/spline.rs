use crate::texture;
use crate::RenderState;
use crate::Vertex;
use crate::world::camera::Camera;

use cgmath::prelude::*;
use cgmath::{Deg, Point3, Vector3};
use winit::event::*;
use winit::keyboard::{Key, NamedKey};
use wgpu::util::DeviceExt;

const SPLINE_SUBDIV: u32 = 16;
const SPLINE_SUBDIV_T: f32 = 1.0 / SPLINE_SUBDIV as f32;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SplineVertex {
    position: [f32; 3],
    t_value: f32,
}

impl SplineVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32];
}

impl crate::Vertex for SplineVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<SplineVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &SplineVertex::ATTRIBS,
        }
    }
}

pub struct Spline {
    // Spline data
    pub points: Vec<SplineControlPoint>,
    pub radius: f32,
    pub sides: u32,
    pub selected_point: u32,

    // Representative mesh
    reconstruct_mesh: bool, // So that we only rebuild our mesh after we update the underlying points
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
    selected_point_buffer: wgpu::Buffer,
    selected_point_bind_group: wgpu::BindGroup,
}

impl Spline {
    pub fn new(device: &wgpu::Device, renderer: &SplineRenderer) -> Self {
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Spline Vertex Buffer"),
            size: 0,
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Spline Index Buffer"),
            size: 0,
            usage: wgpu::BufferUsages::INDEX,
            mapped_at_creation: false,
        });

        // Need to store this buffer/bind group per spline, since we can't write to a buffer
        // mid-render pass. I think.
        let selected_point_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Selected Point Buffer"),
            contents: bytemuck::cast_slice(&[0.0 as f32]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let selected_point_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &renderer.selected_point_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: selected_point_buffer.as_entire_binding(),
                }
            ],
            label: Some("selected_point_bind_group"),
        });


        Spline {
            points: Vec::new(),
            radius: 4.0,
            sides: 3,
            selected_point: 0,

            reconstruct_mesh: false,
            vertex_buffer,
            index_buffer,
            index_count: 0,
            selected_point_buffer,
            selected_point_bind_group,
        }
    }

    pub fn request_rebuild(&mut self) {
        // Update will perform the actual mesh rebuilding
        // For now, we'll just reconstruct the entire mesh on request. We could make this more
        // efficient by only reconstructing the mesh at the updated location. Maybe in the future.
        self.reconstruct_mesh = true;
    }

    pub fn process_events(&mut self, event: &WindowEvent, camera: &Camera) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event: KeyEvent {
                    state,
                    logical_key,
                    repeat,
                    ..
                },
                ..
            } if *state == ElementState::Pressed && *repeat == false => {
                match logical_key.as_ref() {
                    Key::Named(NamedKey::Space) => {
                        let new_point = SplineControlPoint {
                            position: camera.position.map(|c| c.round()),
                            pitch: camera.pitch.into(),
                            yaw: camera.yaw.into(),
                            tangent_magnitude: 1024.0,
                        };
                        if self.selected_point == self.points.len() as u32 {
                            // Append a new control point to the end of the spline at the camera
                            self.points.push(new_point);
                        }
                        else {
                            // Replace the point currently selected with our new point
                            self.points[self.selected_point as usize] = new_point;
                        }
                        self.selected_point += 1;
                        self.request_rebuild();
                        true
                    },
                    Key::Named(NamedKey::ArrowLeft) => {
                        if self.selected_point != 0 {
                            self.selected_point -= 1;
                        }
                        true
                    },
                    Key::Named(NamedKey::ArrowRight) => {
                        if self.selected_point < self.points.len() as u32 {
                            self.selected_point += 1;
                        }
                        true
                    },
                    _ => false,
                }
            }
            _ => false,
        }
    }

    pub fn update(&mut self, render_state: &RenderState) {
        if self.reconstruct_mesh {
            // Start by calculating the positions and tangents of our subdivisions on the spline.
            let mut subdiv_points = Vec::new();
            let mut subdiv_tangents = Vec::new();
            for i in 0..(self.points.len() - 1) {
                for s in 0..SPLINE_SUBDIV {
                    subdiv_points.push(self.points[i].interpolate(&self.points[i + 1], SPLINE_SUBDIV_T * s as f32));

                    let tangent = self.points[i].interp_tangent_dir(&self.points[i + 1], SPLINE_SUBDIV_T * s as f32);
                    subdiv_tangents.push(tangent);
                }
            }
            subdiv_points.push(self.points[self.points.len() - 1].position);
            subdiv_tangents.push(self.points[self.points.len() - 1].calculate_tangent().normalize());

            // Calculate the normals and binormals from the tangents of each subdivision.
            // We calculate the rotation-minimizing (Bishop) frame using the double reflection method:
            // https://www.microsoft.com/en-us/research/wp-content/uploads/2016/12/Computation-of-rotation-minimizing-frames.pdf
            //
            // NOTE: the RMF is a standard choice of frame, but it might be useful to consider this other
            // method of generating frames to use additional objectives, such as keeping oriented with the Z-axis:
            // https://onlinelibrary.wiley.com/doi/10.1111/cgf.14979
            let mut subdiv_normals = Vec::new();
            let mut subdiv_binormals = Vec::new();
            subdiv_normals.push(Vector3::unit_z().cross(subdiv_tangents[0]).normalize());
            subdiv_binormals.push(subdiv_tangents[0].cross(subdiv_normals[0]));
            for i in 1..subdiv_points.len() {
                let reflection_vector_lh = subdiv_points[i] - subdiv_points[i-1];
                let normal_reflection_lh = subdiv_normals[i-1] - (2.0 / reflection_vector_lh.dot(reflection_vector_lh)) * (reflection_vector_lh.dot(subdiv_normals[i-1])) * reflection_vector_lh;
                let tangent_reflection_lh = subdiv_tangents[i-1] - (2.0 / reflection_vector_lh.dot(reflection_vector_lh)) * (reflection_vector_lh.dot(subdiv_tangents[i-1])) * reflection_vector_lh;

                let reflection_vector_rh = subdiv_tangents[i] - tangent_reflection_lh;
                let normal = normal_reflection_lh - (2.0 / reflection_vector_rh.dot(reflection_vector_rh)) * (reflection_vector_rh.dot(normal_reflection_lh)) * reflection_vector_rh;
                subdiv_normals.push(normal);
                subdiv_binormals.push(subdiv_tangents[i].cross(normal));
            }

            // Calculate the polygon positions for our spline mesh. These positions will lie on the
            // normal plane, and thus can be turned into offsets with the normal and binormal.
            let mut poly_positions = Vec::new();
            for i in 0..self.sides {
                let angle = i as f32 / self.sides as f32 * std::f32::consts::TAU;
                poly_positions.push(angle.sin_cos());
            }

            // Construct the vertices for our mesh
            let mut vertices = Vec::new();
            for i in 0..subdiv_points.len() {
                for poly_pos in poly_positions.iter() {
                    let position = subdiv_points[i] + (poly_pos.0 * subdiv_normals[i] + poly_pos.1 * subdiv_binormals[i]) * self.radius;
                    vertices.push(SplineVertex {
                        position: position.into(),
                        t_value: i as f32 * SPLINE_SUBDIV_T,
                    });
                }
            }

            // Construct our indices to form the mesh
            let mut indices: Vec<u32> = Vec::new();
            // End-cap for our first subdivision
            for i in 1..(self.sides - 1) {
                indices.push(0);
                indices.push(i);
                indices.push(i + 1);
            }
            // Triangles between subdivisions
            for subdiv in 0..(subdiv_points.len() - 1) {
                let base_i = subdiv as u32 * self.sides;
                let next_base_i = (subdiv as u32 + 1) * self.sides;
                for i in 0..self.sides {
                    let next_i = (i + 1) % self.sides;
                    indices.push(base_i + next_i);
                    indices.push(base_i + i);
                    indices.push(next_base_i + next_i);

                    indices.push(base_i + i);
                    indices.push(next_base_i + i);
                    indices.push(next_base_i + next_i);
                }
            }
            // End-cap for our last subdivision
            let end_base_i = (subdiv_points.len() as u32 - 1) * self.sides;
            for i in 1..(self.sides - 1) {
                indices.push(end_base_i);
                indices.push(end_base_i + i);
                indices.push(end_base_i + i + 1);
            }

            // Build our mesh buffers for the GPU
            let vertex_buffer = render_state.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Spline Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let index_buffer = render_state.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Spline Index Buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });

            self.vertex_buffer = vertex_buffer;
            self.index_buffer = index_buffer;
            self.index_count = indices.len() as u32;

            // Rebuild the vertex list
            self.reconstruct_mesh = false;
        }

        render_state.queue.write_buffer(&self.selected_point_buffer, 0, bytemuck::cast_slice(&[self.selected_point as f32]));
    }
}

pub struct SplineControlPoint {
    pub position: Point3<f32>,
    pub pitch: Deg<f32>,
    pub yaw: Deg<f32>,
    pub tangent_magnitude: f32,
}

impl SplineControlPoint {
    fn calculate_tangent(&self) -> Vector3<f32> {
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let tangent_dir = Vector3::new(cos_pitch * cos_yaw, cos_pitch * sin_yaw, sin_pitch);
        tangent_dir * self.tangent_magnitude
    }

    fn interpolate(&self, other: &SplineControlPoint, t: f32) -> Point3<f32> {
        let tangent_s = self.calculate_tangent();
        let tangent_o = other.calculate_tangent();
        let pos_s = self.position.to_vec();
        let pos_o = other.position.to_vec();
        // Interpolate using cubic hermite spline formula for 2 points
        let t2 = t*t;
        let t3 = t*t2;
        Point3::from_vec((2.0*t3 - 3.0*t2 + 1.0) * pos_s + (t3 - 2.0*t2 + t) * tangent_s + (-2.0*t3 + 3.0*t2) * pos_o + (t3 - t2) * tangent_o)
    }

    // Used to calculate tangent for inbetween points
    fn interp_tangent_dir(&self, other: &SplineControlPoint, t: f32) -> Vector3<f32> {
        let tangent_s = self.calculate_tangent();
        let tangent_o = other.calculate_tangent();
        let pos_s = self.position.to_vec();
        let pos_o = other.position.to_vec();
        // Tangent can be calculated as the derivative of our above formula w.r.t t.
        let t2 = t*t;
        ((6.0*t2 - 6.0*t) * (pos_s - pos_o) + (3.0*t2 - 4.0*t + 1.0) * tangent_s + (3.0*t2 - 2.0*t) * tangent_o).normalize()
    }
}

// Struct that handles the rendering of spline instances. Separate from Spline so that we can
// freely draw multiple Splines without maintaining separate copies of our rendering state
pub struct SplineRenderer {
    render_pipeline: wgpu::RenderPipeline,
    selected_point_bind_group_layout: wgpu::BindGroupLayout,
}

impl SplineRenderer {
    pub fn new(render_state: &RenderState, camera_layout: &wgpu::BindGroupLayout) -> Self {
        let shader = render_state.device.create_shader_module(wgpu::include_wgsl!("spline_shader.wgsl"));

        let selected_point_bind_group_layout = render_state.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            label: Some("selected_point_bind_group_layout"),
        });

        let render_pipeline_layout = render_state.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Spline Render Pipeline Layout"),
            bind_group_layouts: &[
                camera_layout,
                &selected_point_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let render_pipeline = render_state.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Spline Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[
                    SplineVertex::desc(),
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: render_state.config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: texture::Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        SplineRenderer {
            render_pipeline,
            selected_point_bind_group_layout,
        }
    }

    pub fn draw<'s>(&'s self, render_pass: &mut wgpu::RenderPass<'s>, camera_bind_group: &'s wgpu::BindGroup, spline: &'s Spline) {
        render_pass.set_pipeline(&self.render_pipeline);

        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_bind_group(1, &spline.selected_point_bind_group, &[]);

        render_pass.set_vertex_buffer(0, spline.vertex_buffer.slice(..));
        render_pass.set_index_buffer(spline.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        render_pass.draw_indexed(0..spline.index_count, 0, 0..1);
    }
}
