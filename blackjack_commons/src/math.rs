// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::{Add, Mul, Sub};

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

pub fn lerp<T>(start: T, end: T, t: f32) -> T
where
    T: Copy + Add<Output = T> + Sub<Output = T> + Mul<f32, Output = T>,
{
    start + (end - start) * t
}
