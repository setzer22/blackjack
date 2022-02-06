use nonmax::NonMaxU32;

use crate::prelude::*;

/// A HalfEdge representation storing the halfedge pointers in contiguous
/// arrays. For each of the main arrays, at position `h` there is the data for
/// halfedge with index `h`.
///
/// This representation is better suited to certain algorithms due to the more
/// succint representation which allows easier concurrent access.
///
/// Besides storage type, there are some representation differences between a
/// [`HalfEdgeMesh`], and a [`CompactMesh`]:
/// - A `HalfEdgeMesh` represents a boundary with a halfedge whose twin exists,
///   but points to a None face, whereas in the `CompactMesh` the twin does not
///   exist (non-existence is encoded as u32::MAX, via NonMaxU32)
/// -
#[derive(Debug)]
pub struct CompactMesh {
    /// Index is either Some(idx) or None. Uses NonMaxU32 to ensure elements are
    /// the same size as `u32`.
    pub twin: Vec<Option<NonMaxU32>>,
    pub next: Vec<u32>,
    pub prev: Vec<u32>,
    pub vert: Vec<u32>,
    pub edge: Vec<u32>,
    pub face: Vec<u32>,
    pub vertex_positions: Vec<Vec3>,
    pub counts: MeshCounts,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MeshCounts {
    /// The number of vertices
    pub num_vertices: usize,
    /// The number of halfedges. Note that this is not the same as the number of
    /// edges times two, because in a CompactMesh, a boundary edge only has a
    /// single halfedge instead of two.
    pub num_halfedges: usize,
    /// The number of edges.
    pub num_edges: usize,
    /// The number of faces
    pub num_faces: usize,
}

impl MeshCounts {
    /// Returns the mesh counts of a compact halfedge after a single iteration
    /// of subdivision. Applies recurrence relation.
    pub fn subdiv(&self) -> Self {
        let h_0 = self.num_halfedges;
        let v_0 = self.num_vertices;
        let f_0 = self.num_faces;
        let e_0 = self.num_edges;

        MeshCounts {
            num_halfedges: h_0 * 4,
            num_faces: h_0,
            num_vertices: v_0 + f_0 + e_0,
            num_edges: 2 * e_0 + h_0,
        }
    }
}

impl CompactMesh {
    /// Constructs a [`CompactMesh`] from a [`HalfEdgeMesh`].
    pub fn from_halfedge(mesh: &HalfEdgeMesh) -> Result<CompactMesh> {
        // Create mappings between ids and indices in the compact arrays. This
        // is necessary because slotmap makes no guarantees about the structure
        // of the indices, and in practice there could be arbitrarily large gaps
        // in it, so there is no way to reuse the information in the keys. This
        // is also safer as it doesn't rely on any implementation details.

        // --- Edge and halfedge mapping ---

        // We need to generate 'virtual' edge ids because the Catmull Clark
        // computation requires them.
        let mut h_id_to_edge =
            slotmap::SecondaryMap::<HalfEdgeId, u32>::with_capacity(mesh.halfedges.capacity());
        let mut edge_id_counter = 0;

        let mut h_id_to_idx =
            slotmap::SecondaryMap::<HalfEdgeId, u32>::with_capacity(mesh.halfedges.capacity());

        // NOTE: We skip halfedges in the boundary because the compact halfedge
        // mesh represents boundaries as a halfedge with no twin, whereas our
        // HalfEdgeMesh uses a representation where the twin exists, but its
        // face is set to None.
        mesh.iter_halfedges()
            .filter(|(_, h)| h.face.is_some())
            .enumerate()
            .for_each(|(idx, (id, _))| {
                h_id_to_idx.insert(id, idx as u32);

                // Generate the 'virtual' edge ids
                let twin = mesh.at_halfedge(id).twin().end();
                if let Some(twin_edge) = h_id_to_edge.get(twin).cloned() {
                    // When the twin of this halfedge already has an edge, use that
                    h_id_to_edge.insert(id, twin_edge);
                } else {
                    // Otherwise, we're the first halfedge of the pair, so
                    // create and insert a new id
                    h_id_to_edge.insert(id, edge_id_counter);
                    edge_id_counter += 1;
                }
            });

        // --- Vertex mapping ---

        let mut v_id_to_idx =
            slotmap::SecondaryMap::<VertexId, u32>::with_capacity(mesh.vertices.capacity());
        mesh.iter_vertices().enumerate().for_each(|(idx, (id, _))| {
            v_id_to_idx.insert(id, idx as u32);
        });

        // --- Face mapping ---

        let mut f_id_to_idx =
            slotmap::SecondaryMap::<FaceId, u32>::with_capacity(mesh.faces.capacity());
        mesh.iter_faces().enumerate().for_each(|(idx, (id, _))| {
            f_id_to_idx.insert(id, idx as u32);
        });

        // --- Generate the compact mesh ---

        let num_halfedges = h_id_to_idx.len();
        let num_vertices = v_id_to_idx.len();
        let num_faces = f_id_to_idx.len();

        let mut twin = Vec::with_capacity(num_halfedges);
        let mut next = Vec::with_capacity(num_halfedges);
        let mut prev = Vec::with_capacity(num_halfedges);
        let mut vert = Vec::with_capacity(num_halfedges);
        let mut edge = Vec::with_capacity(num_halfedges);
        let mut face = Vec::with_capacity(num_halfedges);

        for (h_id, _) in h_id_to_idx.iter() {
            let h = &mesh[h_id];

            match mesh.at_halfedge(h_id).twin().face_or_boundary()? {
                Some(_) => {
                    twin.push(NonMaxU32::new(
                        h_id_to_idx[h.twin.ok_or(anyhow!("No twin"))?],
                    ));
                }
                None => {
                    twin.push(None);
                }
            }
            next.push(h_id_to_idx[h.next.ok_or(anyhow!("No next"))?]);
            prev.push(h_id_to_idx[mesh.at_halfedge(h_id).previous().try_end()?]);
            vert.push(v_id_to_idx[h.vertex.ok_or(anyhow!("No vertex"))?]);
            face.push(f_id_to_idx[h.face.ok_or(anyhow!("No face"))?]);
            edge.push(h_id_to_edge[h_id])
        }

        let vertex_positions = v_id_to_idx
            .iter()
            .map(|(v_id, _)| mesh.vertex_position(v_id))
            .collect();

        Ok(CompactMesh {
            twin,
            next,
            prev,
            vert,
            edge,
            face,
            vertex_positions,
            counts: MeshCounts {
                num_halfedges,
                num_vertices,
                num_faces,
                // NOTE: We increment the counter after adding the edge, so the
                // last value is also the count
                num_edges: edge_id_counter as usize,
            },
        })
    }

    pub fn to_halfedge(&self) -> HalfEdgeMesh {
        let mut mesh = HalfEdgeMesh::default();

        let mut h_idx_to_id = Vec::with_capacity(self.counts.num_halfedges);
        for _ in 0..self.counts.num_halfedges {
            h_idx_to_id.push(mesh.alloc_halfedge(HalfEdge::default()));
        }

        let mut v_idx_to_id = Vec::with_capacity(self.counts.num_vertices);
        for v in 0..self.counts.num_vertices {
            v_idx_to_id.push(mesh.alloc_vertex(self.vertex_positions[v], None));
        }

        let mut f_idx_to_id = Vec::with_capacity(self.counts.num_faces);
        for _ in 0..self.counts.num_faces {
            f_idx_to_id.push(mesh.alloc_face(None));
        }

        for (h, (twin, next, vert, face)) in
            itertools::multizip((&self.twin, &self.next, &self.vert, &self.face)).enumerate()
        {
            let h_id = h_idx_to_id[h];

            let twin_id = twin.map(|idx| h_idx_to_id[idx.get() as usize]);
            let next_id = h_idx_to_id[*next as usize];
            let vert_id = v_idx_to_id[*vert as usize];
            let face_id = f_idx_to_id[*face as usize];

            mesh[h_id] = HalfEdge {
                // If twin id is none, a twin boundary halfedge will be created later
                twin: twin_id,
                next: Some(next_id),
                vertex: Some(vert_id),
                face: Some(face_id),
            };
            mesh[vert_id].halfedge = Some(h_id);
            mesh[face_id].halfedge = Some(h_id);
        }

        // TODO: Fix boundary halfedges, if any

        mesh
    }

    /// See "A HalfEdge Refinement Rule for Parallel Catmull-Clark"
    /// https://onrendering.com/data/papers/catmark/HalfedgeCatmullClark.pdf
    pub fn subdivide_halfedge_refinement(&self) -> CompactMesh {
        use rayon::prelude::*;

        // After subdivision, we have 4 times as many halfedges, exactly.
        let mut new_twin: Vec<Option<NonMaxU32>> = vec![None; self.counts.num_halfedges * 4];
        let mut new_next = vec![0u32; self.counts.num_halfedges * 4];
        let mut new_prev = vec![0u32; self.counts.num_halfedges * 4];
        let mut new_vert = vec![0u32; self.counts.num_halfedges * 4];
        let mut new_edge = vec![0u32; self.counts.num_halfedges * 4];
        let mut new_face = vec![0u32; self.counts.num_halfedges * 4];

        // NOTE: The expressions, when taken literally as described in the paper
        // are not very concurrency-friendly. The code mutates the vectors with
        // expressions like `twin[4h+0] = ...` which implies mutable access to
        // the vector from different threads.
        //
        // Instead of iterating over h, we iterate over mutable chunks of 4
        // values in the new vectors. By calling enumerate() on this iterator of
        // chunks, the index matches the `h` in the expressions from the paper
        // and the provided slices naturally span from 4h+0 to 4h+3

        (
            new_twin.par_chunks_mut(4),
            new_next.par_chunks_mut(4),
            new_prev.par_chunks_mut(4),
            new_vert.par_chunks_mut(4),
            new_edge.par_chunks_mut(4),
            new_face.par_chunks_mut(4),
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(h, (twin, next, prev, vert, edge, face))| {

                // Common expressions used in some of the rules below
                let v_d = self.counts.num_vertices as u32;
                let f_d = self.counts.num_faces as u32;
                let e_d = self.counts.num_edges as u32;
                let h_prev = self.prev[h];


                // (a) Halfedge's twin rule
                twin[0] = self.twin[h]
                    .and_then(|twin_h| NonMaxU32::new(4 * self.next[twin_h.get() as usize] + 3));
                twin[1] = NonMaxU32::new(4 * self.next[h] + 2);
                twin[2] = NonMaxU32::new(4 * self.prev[h] + 1);
                twin[3] = self.twin[self.prev[h] as usize]
                    .and_then(|twin_prev_h| NonMaxU32::new(4 * twin_prev_h.get()));

                //  (b) Halfedge's next rule
                next[0] = (4 * h + 1) as u32;
                next[1] = (4 * h + 2) as u32;
                next[2] = (4 * h + 3) as u32;
                next[3] = (4 * h) as u32;

                // (c) Halfedge's previous rule
                prev[0] = (4 * h + 3) as u32;
                prev[1] = (4 * h) as u32;
                prev[2] = (4 * h + 1) as u32;
                prev[3] = (4 * h + 2) as u32;

                // (d) Halfedge's vertex rule
                vert[0] = self.vert[h];
                vert[1] = v_d + f_d + self.edge[h];
                vert[2] = v_d + self.face[h];
                vert[3] = v_d + f_d + self.edge[h_prev as usize];

                // (e) Halfedge's edge rule
                let h_gt_twin_h = self.twin[h]
                    .map(|twin_h| (h as u32) < twin_h.get())
                    .unwrap_or(true);
                let hp_gt_twin_hp = self.twin[h_prev as usize]
                    .map(|twhin_hp| (h_prev as u32) < twhin_hp.get())
                    .unwrap_or(true);


                edge[0] = if h_gt_twin_h {
                    2 * self.edge[h]
                } else {
                    2 * self.edge[h] + 1
                };
                edge[1] = 2 * e_d + h as u32;
                edge[2] = 2 * e_d + h_prev as u32;
                edge[3] = if hp_gt_twin_hp {
                    2 * self.edge[h_prev as usize] + 1
                } else {
                    2 * self.edge[h_prev as usize]
                };

                // (f) Halfedges's face rule
                face[0] = h as u32;
                face[1] = h as u32;
                face[2] = h as u32;
                face[3] = h as u32;
            });

        CompactMesh {
            twin: new_twin,
            prev: new_prev,
            next: new_next,
            vert: new_vert,
            edge: new_edge,
            face: new_face,
            vertex_positions: todo!(),
            counts: self.counts.subdiv(),
        }
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    #[test]
    pub fn mesh_counts_test() {
        // Results empirically validated by subdividing several meshes in
        // Blender and using the 'Statistics' overlay to obtain the counts

        // A cube, after successive levels of subdivision
        let cube_counts = MeshCounts {
            num_vertices: 8,
            num_halfedges: 24,
            num_edges: 12,
            num_faces: 6,
        };
        let cube_counts_cumulative: Vec<MeshCounts> = (0..4)
            .scan(cube_counts, |acc, _| {
                *acc = acc.subdiv();
                Some(*acc)
            })
            .collect();

        assert_eq!(
            &cube_counts_cumulative,
            &[
                MeshCounts {
                    num_vertices: 26,
                    num_halfedges: 48 * 2,
                    num_edges: 48,
                    num_faces: 24,
                },
                MeshCounts {
                    num_vertices: 98,
                    num_halfedges: 192 * 2,
                    num_edges: 192,
                    num_faces: 96,
                },
                MeshCounts {
                    num_vertices: 386,
                    num_halfedges: 768 * 2,
                    num_edges: 768,
                    num_faces: 384,
                },
                MeshCounts {
                    num_vertices: 1538,
                    num_halfedges: 3072 * 2,
                    num_edges: 3072,
                    num_faces: 1536,
                }
            ]
        );

        // A quad, after successive levels of subdivision
        // -- Unlike the cube, this has some halfedges in the boundary.
        let quad_count = MeshCounts {
            num_vertices: 4,
            num_halfedges: 4,
            num_edges: 4,
            num_faces: 1,
        };

        let quad_counts_cumulative: Vec<MeshCounts> = (0..4)
            .scan(quad_count, |acc, _| {
                *acc = acc.subdiv();
                Some(*acc)
            })
            .collect();
        assert_eq!(
            &quad_counts_cumulative,
            &[
                MeshCounts {
                    num_vertices: 9,
                    num_halfedges: 16,
                    num_edges: 12,
                    num_faces: 4,
                },
                MeshCounts {
                    num_vertices: 25,
                    num_halfedges: 64,
                    num_edges: 40,
                    num_faces: 16,
                },
                MeshCounts {
                    num_vertices: 81,
                    num_halfedges: 256,
                    num_edges: 144,
                    num_faces: 64,
                },
                MeshCounts {
                    num_vertices: 289,
                    num_halfedges: 1024,
                    num_edges: 544,
                    num_faces: 256,
                },
            ]
        );
    }
}
