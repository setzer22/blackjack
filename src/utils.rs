use crate::prelude::*;

pub trait IteratorUtils: Iterator {
    fn collect_svec(self) -> SVec<Self::Item>
    where
        Self: Sized,
    {
        self.collect()
    }
}

/// Rotates the given iterator by shifting all elements `shift` positions
/// forward. Any elements that would be out of bounds are instead put at the
/// beginning. 
/// 
/// This method requires passing the `len` as a separate parameter. This is
/// often known beforehand or can be found by calling .size_hint() for an
/// ExactSizeIterator.
pub fn rotate_iter<T>(
    it: impl Iterator<Item = T> + Clone,
    shift: usize,
    len: usize,
) -> impl Iterator<Item = T> {
    it.cycle().dropping(shift).take(len)
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
