use cgmath::prelude::*;
use cgmath::{Deg, Point3, Matrix4, Rad, Vector3};
use std::f32::consts::FRAC_PI_2;
use web_time::Duration;
use winit::event::*;
use winit::keyboard::Key;

const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().into();
    }

}

#[rustfmt::skip]
const OPENGL_TO_WGPU_MATRIX: Matrix4<f32> = Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);

pub struct Camera {
    pub position: Point3<f32>,
    pub pitch: Rad<f32>,
    pub yaw: Rad<f32>,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Camera {
    pub fn build_view_projection_matrix(&self) -> Matrix4<f32> {
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let view_dir = Vector3::new(cos_pitch * cos_yaw, cos_pitch * sin_yaw, sin_pitch);

        let view = Matrix4::look_to_rh(self.position, view_dir, Vector3::unit_z());
        let proj = cgmath::perspective(Deg(self.fovy), self.aspect, self.znear, self.zfar);

        return OPENGL_TO_WGPU_MATRIX * proj * view;
    }
}

pub struct CameraController {
    speed: f32,
    speed_multiplier: f32,
    sensitivity: f32,
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
    is_speed_multiplied: bool,
    delta_pitch: f32,
    delta_yaw: f32,
}

impl CameraController {
    // Speed is given in units per second, sensitivity in rad per pixel
    pub fn new(speed: f32, speed_multiplier:f32, sensitivity: f32) -> Self {
        Self {
            speed,
            speed_multiplier,
            sensitivity,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
            is_speed_multiplied: false,
            delta_pitch: 0.0,
            delta_yaw: 0.0,
        }
    }

    pub fn process_events(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event: KeyEvent {
                    state,
                    logical_key,
                    ..
                },
                ..
            } => {
                let is_pressed = *state == ElementState::Pressed;
                match logical_key.as_ref() {
                    Key::Character("w") => {
                        self.is_forward_pressed = is_pressed;
                        self.is_speed_multiplied = false;
                        true
                    }
                    Key::Character("W") => {
                        self.is_forward_pressed = is_pressed;
                        self.is_speed_multiplied = true;
                        true
                    }
                    Key::Character("a") => {
                        self.is_left_pressed = is_pressed;
                        self.is_speed_multiplied = false;
                        true
                    }
                    Key::Character("A") => {
                        self.is_left_pressed = is_pressed;
                        self.is_speed_multiplied = true;
                        true
                    }
                    Key::Character("s") => {
                        self.is_backward_pressed = is_pressed;
                        self.is_speed_multiplied = false;
                        true
                    }
                    Key::Character("S") => {
                        self.is_backward_pressed = is_pressed;
                        self.is_speed_multiplied = true;
                        true
                    }
                    Key::Character("d") => {
                        self.is_right_pressed = is_pressed;
                        self.is_speed_multiplied = false;
                        true
                    }
                    Key::Character("D") => {
                        self.is_right_pressed = is_pressed;
                        self.is_speed_multiplied = true;
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    pub fn process_mouse(&mut self, delta: (f64, f64)) {
        // Need to filter out updates where the mouse didn't move because some web browsers will
        // send a "blank" mouse update (at least in terms of delta) for each normal update.
        if delta.0 != 0.0 || delta.1 != 0.0 {
            self.delta_pitch = -delta.1 as f32;
            self.delta_yaw = -delta.0 as f32;
        }
    }

    pub fn update_camera(&mut self, camera: &mut Camera, dt: Duration) {
        let dt = dt.as_secs_f32();
        let mut speed = self.speed;
        if self.is_speed_multiplied {
            speed *= self.speed_multiplier;
        }

        let (sin_pitch, cos_pitch) = camera.pitch.sin_cos();
        let (sin_yaw, cos_yaw) = camera.yaw.sin_cos();
        let view_dir = Vector3::new(cos_pitch * cos_yaw, cos_pitch * sin_yaw, sin_pitch);

        // For forwards/backwards, we translate in the direction of the camera
        if self.is_forward_pressed {
            camera.position += view_dir * speed * dt;
        }
        if self.is_backward_pressed {
            camera.position -= view_dir * speed * dt;
        }

        // Since we don't have any roll, left/right will always be in the z = 0 plane.
        let view_right = Vector3::new(sin_yaw, -cos_yaw, 0.0);

        if self.is_right_pressed {
            camera.position += view_right * speed * dt;
        }
        if self.is_left_pressed {
            camera.position -= view_right * speed * dt;
        }

        camera.pitch += Rad(self.delta_pitch * self.sensitivity);
        camera.yaw += Rad(self.delta_yaw * self.sensitivity);

        self.delta_pitch = 0.0;
        self.delta_yaw = 0.0;

        if camera.pitch < -Rad(SAFE_FRAC_PI_2) {
            camera.pitch = -Rad(SAFE_FRAC_PI_2);
        }
        else if camera.pitch > Rad(SAFE_FRAC_PI_2) {
            camera.pitch = Rad(SAFE_FRAC_PI_2);
        }
    }
}
