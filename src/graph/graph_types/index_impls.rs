use super::*;

macro_rules! impl_index_traits {
    ($id_type:ty, $output_type:ty, $arena:ident) => {
        impl std::ops::Index<$id_type> for Graph {
            type Output = $output_type;

            fn index(&self, index: $id_type) -> &Self::Output {
                self.$arena.get(index).unwrap_or_else(|| {
                    panic!(
                        "{} index error for {:?}. Has the value been deleted?",
                        stringify!($id_type),
                        index
                    )
                })
            }
        }

        impl std::ops::IndexMut<$id_type> for Graph {
            fn index_mut(&mut self, index: $id_type) -> &mut Self::Output {
                self.$arena.get_mut(index).unwrap_or_else(|| {
                    panic!(
                        "{} index error for {:?}. Has the value been deleted?",
                        stringify!($id_type),
                        index
                    )
                })
            }
        }
    };
}

impl_index_traits!(NodeId, Node, nodes);
impl_index_traits!(InputId, InputParam, inputs);
impl_index_traits!(OutputId, OutputParam, outputs);
