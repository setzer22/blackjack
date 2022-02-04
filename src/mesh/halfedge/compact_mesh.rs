use std::num::NonZeroU32;

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
    pub vertex_positions: Vec<Vec3>,

    pub num_halfedges: usize,
    pub num_vertices: usize,
    pub num_faces: usize,
}

impl CompactMesh {
    pub fn from_halfedge(mesh: &HalfEdgeMesh) -> Result<CompactMesh> {
        let mut twin_vec = vec![];
        let mut next_vec = vec![];
        let mut prev_vec = vec![];
        let mut vert_vec = vec![];
        let mut face_vec = vec![];

        // HACK: Here we rely on a very specific implementation detail from
        // `slotmap`, namely, the index values stored inside KeyData are 1 more
        // than the index of the value in the array (so first element has index
        // 1, and so on). This is why the code below subtracts 1 from each index.

        for (h_id, h) in mesh.iter_halfedges() {
            twin_vec.push(h.twin.ok_or_else(|| anyhow!("Halfedge has no twin"))?.idx() as u32 - 1);
            next_vec.push(h.next.ok_or_else(|| anyhow!("Halfedge has no next"))?.idx() as u32 - 1);
            prev_vec.push(
                mesh.at_halfedge(h_id)
                    .previous()
                    .try_end()
                    .map_err(|_| anyhow!("Halfedge has no previous"))?
                    .idx() as u32
                    - 1,
            );
            vert_vec.push(
                h.vertex
                    .ok_or_else(|| anyhow!("Halfedge has no vertex"))?
                    .idx() as u32
                    - 1,
            );
            // Boundaries (face == None) are encoded as u32::MAX
            face_vec.push(h.face.map(|x| x.idx() as u32 - 1).unwrap_or(u32::MAX));
        }

        Ok(CompactMesh {
            twin: twin_vec,
            next: next_vec,
            prev: prev_vec,
            vert: vert_vec,
            face: face_vec,
            vertex_positions: mesh.iter_vertices().map(|(_, v)| v.position).collect(),
            num_halfedges: mesh.num_halfedges(),
            num_vertices: mesh.num_vertices(),
            num_faces: mesh.num_faces(),
        })
    }

    pub fn to_halfedge(&self) -> HalfEdgeMesh {
        let mut halfedges =
            slotmap::SlotMap::<HalfEdgeId, HalfEdge>::with_capacity_and_key(self.num_halfedges);
        let mut vertices =
            slotmap::SlotMap::<VertexId, Vertex>::with_capacity_and_key(self.num_vertices);
        let mut faces = slotmap::SlotMap::<FaceId, Face>::with_capacity_and_key(self.num_vertices);

        let version = NonZeroU32::new(1).unwrap();


        // HACK: To undo the transformation in the function above, we add 1 to
        // each slotmap index to ensure the keys are well formed.
        let mk_h_id =
            |idx: u32| HalfEdgeId::from(unsafe { slotmap::KeyData::from_raw(idx + 1, version) });
        let mk_v_id =
            |idx: u32| VertexId::from(unsafe { slotmap::KeyData::from_raw(idx + 1, version) });
        let mk_f_id = // -
            |idx: u32| FaceId::from(unsafe { slotmap::KeyData::from_raw(idx + 1, version) });

        // Stores the mapping between vertices and halfedges. This can then be
        // used to iterate in order to fill the Vertices slotmap.
        let mut v_to_h_id = vec![None; self.num_vertices];
        let mut f_to_h_id = vec![None; self.num_faces];

        for h in 0..self.num_halfedges {
            let h_id = mk_h_id(h as u32);
            let real_h_id = halfedges.insert(HalfEdge {
                twin: Some(mk_h_id(self.twin[h])),
                next: Some(mk_h_id(self.next[h])),
                vertex: Some(mk_v_id(self.vert[h])),
                face: {
                    let f = self.face[h];
                    if f == u32::MAX {
                        None
                    } else {
                        Some(mk_f_id(f))
                    }
                },
            });

            // This makes sure the ids we're crafting out of thin air are what
            // we expect them to be. Our assumption here is that an slotmap will
            // hand out keys in a sequential fashion, starting from index 0
            debug_assert_eq!(h_id, real_h_id, "Halfedge ids should be equal");

            v_to_h_id[self.vert[h] as usize] = Some(h_id);
            if self.face[h] != u32::MAX {
                f_to_h_id[self.face[h] as usize] = Some(h_id);
            }
        }

        for v in 0..self.num_vertices {
            let halfedge = v_to_h_id[v];
            debug_assert!(
                halfedge.is_some(),
                "All vertices should point to a halfedge"
            );
            let position = self.vertex_positions[v];

            let v_id = vertices.insert(Vertex { position, halfedge });

            debug_assert_eq!(v_id, mk_v_id(v as u32), "Vertex ids should be equal");
        }

        for f in 0..self.num_faces {
            let halfedge = f_to_h_id[f];
            debug_assert!(halfedge.is_some(), "All faces should point to a halfedge");
            let f_id = faces.insert(Face { halfedge });
            debug_assert_eq!(f_id, mk_f_id(f as u32), "Face ids should be equal");
        }

        HalfEdgeMesh {
            vertices,
            faces,
            halfedges,
            debug_edges: Default::default(),
            debug_vertices: Default::default(),
        }
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    #[test]
    pub fn test() {
        let round_trip =
            CompactMesh::from_halfedge(&super::primitives::Box::build(Vec3::ZERO, Vec3::ONE))
                .unwrap()
                .to_halfedge();

        dbg!(CompactMesh::from_halfedge(&super::primitives::Box::build(
            Vec3::ZERO,
            Vec3::ONE
        )));
    }
}
