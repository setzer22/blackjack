use float_ord::FloatOrd;
use winit::dpi::PhysicalPosition;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Vec3Ord([FloatOrd<f32>; 3]);

pub trait ToOrd<T>
where
    T: Eq + PartialEq + Ord + PartialOrd + std::hash::Hash + Copy,
{
    fn to_ord(&self) -> T;
}

impl ToOrd<Vec3Ord> for glam::Vec3 {
    fn to_ord(&self) -> Vec3Ord {
        Vec3Ord([FloatOrd(self.x), FloatOrd(self.y), FloatOrd(self.z)])
    }
}

pub trait ToVec<T> {
    fn to_vec(&self) -> T;
}

impl ToVec<glam::Vec3> for Vec3Ord {
    fn to_vec(&self) -> glam::Vec3 {
        glam::Vec3::new(self.0[0].0, self.0[1].0, self.0[2].0)
    }
}

pub trait ToEgui<Out> {
    fn to_egui(self) -> Out;
}

impl ToEgui<egui::Pos2> for PhysicalPosition<f64> {
    fn to_egui(self) -> egui::Pos2 {
        egui::pos2(self.x as f32, self.y as f32)
    }
}

pub trait ToWinit<Out> {
    fn to_winit(self) -> Out;
}

impl ToWinit<winit::dpi::PhysicalPosition<f64>> for egui::Pos2 {
    fn to_winit(self) -> winit::dpi::PhysicalPosition<f64> {
        winit::dpi::PhysicalPosition {
            x: self.x.into(),
            y: self.y.into(),
        }
    }
}

pub trait ColorUtils {
    /// Multiplies the color rgb values by `factor`, keeping alpha untouched.
    fn lighten(&self, factor: f32) -> Self;
}

impl ColorUtils for egui::Color32 {
    fn lighten(&self, factor: f32) -> Self {
        egui::Color32::from_rgba_premultiplied(
            (self.r() as f32 * factor) as u8,
            (self.g() as f32 * factor) as u8,
            (self.b() as f32 * factor) as u8,
            self.a(),
        )
    }
}

pub trait RectUtils {
    /// Scales the rect by the given `scale` factor relative to the origin at (0,0)
    fn scale_from_origin(&self, scale: f32) -> Self;
}

impl RectUtils for egui::Rect {
    fn scale_from_origin(&self, scale: f32) -> Self {
        let mut result = *self;
        result.min = (self.min.to_vec2() * scale).to_pos2();
        result.max = (self.max.to_vec2() * scale).to_pos2();
        result
    }
}
