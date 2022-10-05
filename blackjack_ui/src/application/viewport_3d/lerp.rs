use std::ops::{Sub, Mul, Add, AddAssign};

/// A generic lerper. Calling `get`, returns the current value, but calling
/// `set` (or any update methods like AddAssign) update the target value.
///
/// You need to call the lerp's update function each frame to bring the current
/// value closer to the target.
pub struct Lerp<T> {
    current: T,
    target: T,
}

impl<T> Lerp<T>
where
    T: Add<Output = T> + Sub<Output = T> + Mul<f32, Output = T> + Copy,
{
    pub fn new(v: T) -> Self {
        Self {
            current: v,
            target: v,
        }
    }

    pub fn set(&mut self, f: impl Fn(T) -> T) {
        self.target = f(self.target);
    }

    pub fn get(&self) -> T {
        self.current
    }

    pub fn update(&mut self, delta: f32) {
        self.current = self.current + (self.target - self.current) * delta
    }
}

impl<T> AddAssign<T> for Lerp<T>
where
    T: AddAssign<T>,
{
    fn add_assign(&mut self, rhs: T) {
        self.target += rhs;
    }
}