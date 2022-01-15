#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HalfEdgeId(pub(super) generational_arena::Index);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VertexId(pub(super) generational_arena::Index);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FaceId(pub(super) generational_arena::Index);

impl From<HalfEdgeId> for usize {
    fn from(v: HalfEdgeId) -> Self {
        v.0.into_raw_parts().0
    }
}

impl From<FaceId> for usize {
    fn from(v: FaceId) -> Self {
        v.0.into_raw_parts().0
    }
}

impl From<VertexId> for usize {
    fn from(v: VertexId) -> Self {
        v.0.into_raw_parts().0
    }
}

impl HalfEdgeId {
    pub fn idx(&self) -> usize {
        self.0.into_raw_parts().0
    }
}

impl VertexId {
    pub fn idx(&self) -> usize {
        self.0.into_raw_parts().0
    }
}

impl FaceId {
    pub fn idx(&self) -> usize {
        self.0.into_raw_parts().0
    }
}

impl PartialOrd for HalfEdgeId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.idx().partial_cmp(&other.idx())
    }
}
impl Ord for HalfEdgeId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.idx().cmp(&other.idx())
    }
}

impl PartialOrd for VertexId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.idx().partial_cmp(&other.idx())
    }
}
impl Ord for VertexId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.idx().cmp(&other.idx())
    }
}

impl PartialOrd for FaceId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.idx().partial_cmp(&other.idx())
    }
}
impl Ord for FaceId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.idx().cmp(&other.idx())
    }
}