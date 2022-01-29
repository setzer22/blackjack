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

impl ToEgui<egui::Pos2> for PhysicalPosition<f64>
{
    fn to_egui(self) -> egui::Pos2 {
        egui::pos2(self.x as f32, self.y as f32)
    }
}

pub trait ToWinit<Out> {
    fn to_winit(self) -> Out;
}

impl ToWinit<winit::dpi::PhysicalPosition<f64>> for egui::Pos2
where
{
    fn to_winit(self) -> winit::dpi::PhysicalPosition<f64> {
        winit::dpi::PhysicalPosition {
            x: self.x.into(),
            y: self.y.into(),
        }
    }
}
