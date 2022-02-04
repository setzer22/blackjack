use crate::prelude::*;

pub trait IteratorUtils: Iterator {
    fn collect_svec(self) -> SVec<Self::Item>
    where
        Self: Sized,
    {
        self.collect()
    }
}

impl<T: ?Sized> IteratorUtils for T where T: Iterator {}

pub trait SliceUtils<T> {
    /// Same as .iter().copied(), but doesn't trigger rustfmt line breaks
    fn iter_cpy(&self) -> std::iter::Copied<std::slice::Iter<'_, T>>;
}

impl<T: Copy> SliceUtils<T> for [T] {
    fn iter_cpy(&self) -> std::iter::Copied<std::slice::Iter<'_, T>> {
        self.iter().copied()
    }
}
