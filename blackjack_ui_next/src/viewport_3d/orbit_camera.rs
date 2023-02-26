use epaint::Vec2;
use glam::{Mat4, UVec2, Vec3};

use crate::renderer::ViewportCamera;

use super::lerp::Lerp;

pub struct OrbitCamera {
    yaw: Lerp<f32>,
    pitch: Lerp<f32>,
    distance: Lerp<f32>,
    focus_point: Lerp<Vec3>,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            yaw: Lerp::new(-30.0),
            pitch: Lerp::new(30.0),
            distance: Lerp::new(8.0),
            focus_point: Lerp::new(Vec3::ZERO),
        }
    }
}

impl OrbitCamera {
    pub fn update(&mut self, delta: f32) {
        self.yaw.update(delta);
        self.pitch.update(delta);
        self.distance.update(delta);
        self.focus_point.update(delta);
    }

    pub fn on_input(
        &mut self,
        lmb_pressed: bool,
        shift_down: bool,
        cursor_delta: Vec2,
        wheel_delta: f32,
    ) {
        const MIN_DIST: f32 = 0.1;
        const MAX_DIST: f32 = 120.0;

        if lmb_pressed {
            if shift_down {
                let cam_rotation = Mat4::from_rotation_y(self.yaw.get().to_radians())
                    * Mat4::from_rotation_x(self.pitch.get().to_radians());
                let camera_right = cam_rotation.transform_point3(Vec3::X);
                let camera_up = cam_rotation.transform_vector3(Vec3::Y);
                let move_speed = self.distance.get() / MAX_DIST;
                self.focus_point += cursor_delta.x * camera_right * move_speed
                    + cursor_delta.y * -camera_up * move_speed;
            } else {
                self.yaw += cursor_delta.x * 2.0;
                self.pitch += cursor_delta.y * 2.0;
            }
        }
        self.distance
            .set(|dist| (dist - wheel_delta * 0.5).clamp(MIN_DIST, MAX_DIST));
    }

    /// Returns the view matrix and projection matrix for this camera
    pub fn compute_matrices(&self, resolution: UVec2) -> ViewportCamera {
        ViewportCamera {
            view_matrix: Mat4::from_translation(Vec3::Z * self.distance.get())
                * Mat4::from_rotation_x(-self.pitch.get().to_radians())
                * Mat4::from_rotation_y(-self.yaw.get().to_radians())
                * Mat4::from_translation(self.focus_point.get()),
            projection_matrix: glam::Mat4::perspective_infinite_reverse_lh(
                60.0f32.to_radians(),
                resolution.x as f32 / resolution.y as f32,
                0.01,
            ),
        }
    }
}
