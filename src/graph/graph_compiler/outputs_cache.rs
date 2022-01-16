use type_map::TypeMap;

use crate::{prelude::graph::*, prelude::*};
use crate::graph::poly_asm::MemAddr;

/// During compilation, it is necessary to map output parameter ids to the
/// memory addresses where those outputs will be stored. Since memory addresses
/// are typed, we cannot use a regular HashMap, so use use a wrapper TypeMap to
/// store multiple HashMaps per address type.
#[derive(Default)]
pub struct OutputsCache {
    inner: TypeMap,
}
impl OutputsCache {
    pub fn insert<T: Clone + Send + Sync + 'static>(
        &mut self,
        param_id: OutputId,
        addr: MemAddr<T>,
    ) {
        let cache: &mut HashMap<OutputId, MemAddr<T>> =
            self.inner.entry().or_insert_with(|| Default::default());
        cache.insert(param_id, addr);
    }

    pub fn get<T: Clone + Send + Sync + 'static>(
        &mut self,
        param_id: OutputId,
    ) -> Option<MemAddr<T>> {
        let cache: &mut HashMap<OutputId, MemAddr<T>> =
            self.inner.entry().or_insert_with(|| Default::default());
        cache.get(&param_id).map(|x| *x)
    }
}
