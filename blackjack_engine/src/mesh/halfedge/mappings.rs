use std::ops::Index;

use slotmap::SecondaryMap;

pub struct MeshMapping<K: slotmap::Key>(pub SecondaryMap<K, u32>);
impl<K: slotmap::Key> Index<K> for MeshMapping<K> {
    type Output = u32;
    fn index(&self, index: K) -> &Self::Output {
        &self.0[index]
    }
}

impl<K: slotmap::Key> MeshMapping<K> {
    pub fn map_seq(&self, seq: &[K]) -> Vec<u32> {
        seq.iter().map(|x| self[*x]).collect()
    }
}
