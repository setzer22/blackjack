use glam::{Mat4, Vec2, Vec3};

use crate::renderer::ViewportCamera;

#[derive(Clone, Copy, Debug, Default)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Plane {
    pub point: Vec3,
    pub normal: Vec3,
}

impl Ray {
    /// Construct a ray from a point in screenspace and a camera.
    pub fn from_screenspace(
        camera: &ViewportCamera,
        cursor_pos_screen: impl IntoGlam<Vec2>,
        screen_size: impl IntoGlam<Vec2>,
    ) -> Ray {
        let mut cursor_pos_screen: Vec2 = cursor_pos_screen.into_glam();
        let screen_size: Vec2 = screen_size.into_glam();

        // Compute the inverse of the view and projection matrices
        let inv_view = camera.view_matrix.inverse();
        let inv_projection = camera.projection_matrix.inverse();

        // Normalized device coordinate cursor position from (-1, -1, -1) to (1, 1, 1)
        cursor_pos_screen.y = screen_size.y - cursor_pos_screen.y;
        let cursor_ndc = (cursor_pos_screen / screen_size) * 2.0 - Vec2::new(1.0, 1.0);
        let cursor_pos_ndc_near: Vec3 = cursor_ndc.extend(-1.0);
        let cursor_pos_ndc_far: Vec3 = cursor_ndc.extend(1.0);

        // Use near and far ndc points to generate a ray in world space
        let ndc_to_world: Mat4 = inv_view * inv_projection;
        let cursor_pos_near: Vec3 = ndc_to_world.project_point3(cursor_pos_ndc_near);
        let cursor_pos_far: Vec3 = ndc_to_world.project_point3(cursor_pos_ndc_far);
        let ray_direction = (cursor_pos_far - cursor_pos_near).normalize();

        Ray {
            origin: cursor_pos_near,
            direction: ray_direction,
        }
    }

    /// Returns the point of intersection between the ray and the plane. Or None
    /// if there is no intersection (because the plane and ray are parallel).
    pub fn intersect_plane(&self, plane: &Plane) -> Option<Vec3> {
        let denom = self.direction.dot(plane.normal);
        if denom.abs() > 1e-6 {
            let t = (plane.point - self.origin).dot(plane.normal) / denom;
            if t >= 0.0 {
                return Some(self.origin + self.direction * t);
            }
        }
        None
    }

    /// Computes the closest point between this ray and a line. This is similar
    /// to projecting onto a plane, but restricting the result onto a line.
    ///
    /// https://math.stackexchange.com/a/4473496
    pub fn closest_point_to_line(&self, line: &Ray) -> Vec3 {
        let line_position = line.origin;
        let line_normal = line.direction;
        let ray_position = self.origin;
        let ray_normal = self.direction;

        let pos_diff = line_position - ray_position;
        let cross_normal = line_normal.cross(ray_normal).normalize();
        let rejection =
            pos_diff - pos_diff.project_onto(ray_normal) - pos_diff.project_onto(cross_normal);
        let distance_to_line_pos = rejection.length() / line_normal.dot(rejection.normalize());
        let closest_approach = line_position - line_normal * distance_to_line_pos;
        return closest_approach;
    }
}

/// A trait to convert from epaint types to glam types.
pub trait IntoGlam<T> {
    fn into_glam(self) -> T;
}

impl IntoGlam<Vec2> for epaint::Vec2 {
    fn into_glam(self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }
}

impl IntoGlam<Vec2> for epaint::Pos2 {
    fn into_glam(self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }
}

impl<T> IntoGlam<T> for T {
    fn into_glam(self) -> T {
        self
    }
}
