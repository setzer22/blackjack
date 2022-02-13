use super::*;

macro_rules! impl_index_traits {
    ($id_type:ty, $output_type:ty, $arena:ident) => {
        impl std::ops::Index<$id_type> for HalfEdgeMesh {
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

        impl std::ops::IndexMut<$id_type> for HalfEdgeMesh {
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

impl_index_traits!(VertexId, Vertex, vertices);
impl_index_traits!(FaceId, Face, faces);
impl_index_traits!(HalfEdgeId, HalfEdge, halfedges);

macro_rules! impl_index_ops {
    ($field_name:ident, $field_name_mut:ident, $id_type:ty, $output_type:ty, $arena:ident) => {
        /// Try to immutably borrow data
        pub fn $field_name(&self, id: $id_type) -> Option<&$output_type> {
            self.$arena.get(id)
        }

        /// Try to mutably borrow data
        pub fn $field_name_mut(&mut self, id: $id_type) -> Option<&mut $output_type> {
            self.$arena.get_mut(id)
        }
    };
}

impl HalfEdgeMesh {
    impl_index_ops!(vertex, vertex_mut, VertexId, Vertex, vertices);
    impl_index_ops!(face, face_mut, FaceId, Face, faces);
    impl_index_ops!(halfedge, halfedge_mut, HalfEdgeId, HalfEdge, halfedges);
}
