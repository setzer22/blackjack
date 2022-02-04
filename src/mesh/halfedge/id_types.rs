slotmap::new_key_type! { pub struct HalfEdgeId; }
slotmap::new_key_type! { pub struct VertexId; }
slotmap::new_key_type! { pub struct FaceId; }

impl From<HalfEdgeId> for usize {
    fn from(v: HalfEdgeId) -> Self {
        v.0.idx() as usize
    }
}

impl From<FaceId> for usize {
    fn from(v: FaceId) -> Self {
        v.0.idx() as usize
    }
}

impl From<VertexId> for usize {
    fn from(v: VertexId) -> Self {
        v.0.idx() as usize
    }
}

impl HalfEdgeId {
    pub fn idx(&self) -> usize {
        self.0.idx() as usize
    }
}

impl VertexId {
    pub fn idx(&self) -> usize {
        self.0.idx() as usize
    }
}

impl FaceId {
    pub fn idx(&self) -> usize {
        self.0.idx() as usize
    }
}
