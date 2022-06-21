use float_ord::FloatOrd;

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
