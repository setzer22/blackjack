use crate::prelude::*;

/// A HalfEdge representation storing the halfedge pointers in contiguous
/// arrays. Indices are u32. Halfedge vector is implicitly 0..n
#[derive(Debug)]
pub struct CompactMesh {
    pub twin: Vec<u32>,
    pub next: Vec<u32>,
    pub prev: Vec<u32>,
    pub vert: Vec<u32>,
    pub face: Vec<u32>,
    pub vertices: Vec<Vec3>,
}

impl CompactMesh {
    pub fn from_halfedge(mesh: &HalfEdgeMesh) -> Result<CompactMesh> {
        let mut twin_vec = vec![];
        let mut next_vec = vec![];
        let mut prev_vec = vec![];
        let mut vert_vec = vec![];
        let mut face_vec = vec![];

        for (h_id, h) in mesh.iter_halfedges() {
            twin_vec.push(h.twin.ok_or_else(|| anyhow!("Halfedge has no twin"))?.idx() as u32);
            next_vec.push(h.next.ok_or_else(|| anyhow!("Halfedge has no next"))?.idx() as u32);
            prev_vec.push(
                mesh.at_halfedge(h_id)
                    .previous()
                    .try_end()
                    .map_err(|_| anyhow!("Halfedge has no previous"))?
                    .idx() as u32,
            );
            vert_vec.push(
                h.vertex
                    .ok_or_else(|| anyhow!("Halfedge has no vertex"))?
                    .idx() as u32,
            );
            face_vec.push(
                h.vertex
                    .ok_or_else(|| anyhow!("Halfedge has no face"))?
                    .idx() as u32,
            );
        }

        Ok(CompactMesh {
            twin: twin_vec,
            next: next_vec,
            prev: prev_vec,
            vert: vert_vec,
            face: face_vec,
            vertices: mesh.iter_vertices().map(|(_, v)| v.position).collect(),
        })
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    #[test]
    pub fn test() {
        let cmesh = CompactMesh::from_halfedge(&super::primitives::Box::build(Vec3::ZERO, Vec3::ONE));
        dbg!(cmesh);
    }
}
