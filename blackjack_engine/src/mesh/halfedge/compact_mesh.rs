// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use atomic_float::AtomicF32;
use nonmax::NonMaxU32;
use std::sync::atomic::Ordering;

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
///
/// There is a const parameter, `Subdivided` that indicates whether this mesh
/// has been subdivided. This is useful because the subdivision algorithm can
/// substantially speed up successive subdivisions for all iterations but the
/// first. We use a const generic to make sure rust will monomorphize the calls
/// that use the Subdivided parameter to create specialized versions of the
/// code.
#[derive(Debug)]
#[allow(non_upper_case_globals)]
pub struct CompactMesh<const Subdivided: bool> {
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

#[allow(non_upper_case_globals)]
impl<const Subdivided: bool> CompactMesh<Subdivided> {
    #[profiling::function]
    pub fn from_halfedge(mesh: &HalfEdgeMesh) -> Result<CompactMesh<false>> {
        let conn = mesh.read_connectivity();

        // Create mappings between ids and indices in the compact arrays. This
        // is necessary because slotmap makes no guarantees about the structure
        // of the indices, and in practice there could be arbitrarily large gaps
        // in it, so there is no way to reuse the information in the keys. This
        // is also safer as it doesn't rely on any implementation details.

        // --- Edge and halfedge mapping ---

        // We need to generate 'virtual' edge ids because the Catmull Clark
        // computation requires them.
        let mut h_id_to_edge =
            slotmap::SecondaryMap::<HalfEdgeId, u32>::with_capacity(conn.halfedges.capacity());
        let mut edge_id_counter = 0;

        let mut h_id_to_idx =
            slotmap::SecondaryMap::<HalfEdgeId, u32>::with_capacity(conn.halfedges.capacity());

        // NOTE: We skip halfedges in the boundary because the compact halfedge
        // mesh represents boundaries as a halfedge with no twin, whereas our
        // HalfEdgeMesh uses a representation where the twin exists, but its
        // face is set to None.
        conn.iter_halfedges()
            .filter(|(_, h)| h.face.is_some())
            .enumerate()
            .for_each(|(idx, (id, _))| {
                h_id_to_idx.insert(id, idx as u32);

                // Generate the 'virtual' edge ids
                let twin = conn.at_halfedge(id).twin().end();
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
            slotmap::SecondaryMap::<VertexId, u32>::with_capacity(conn.vertices.capacity());
        conn.iter_vertices().enumerate().for_each(|(idx, (id, _))| {
            v_id_to_idx.insert(id, idx as u32);
        });

        // --- Face mapping ---

        let mut f_id_to_idx =
            slotmap::SecondaryMap::<FaceId, u32>::with_capacity(conn.faces.capacity());
        conn.iter_faces().enumerate().for_each(|(idx, (id, _))| {
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
            let h = &conn[h_id];

            match conn.at_halfedge(h_id).twin().face_or_boundary()? {
                Some(_) => {
                    twin.push(NonMaxU32::new(
                        h_id_to_idx[h.twin.ok_or_else(|| anyhow!("No twin"))?],
                    ));
                }
                None => {
                    twin.push(None);
                }
            }
            next.push(h_id_to_idx[h.next.ok_or_else(|| anyhow!("No next"))?]);
            prev.push(h_id_to_idx[conn.at_halfedge(h_id).previous().try_end()?]);
            vert.push(v_id_to_idx[h.vertex.ok_or_else(|| anyhow!("No vertex"))?]);
            face.push(f_id_to_idx[h.face.ok_or_else(|| anyhow!("No face"))?]);
            edge.push(h_id_to_edge[h_id])
        }

        let positions = mesh.read_positions();
        let vertex_positions = v_id_to_idx
            .iter()
            .map(|(v_id, _)| positions[v_id])
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

    #[profiling::function]
    pub fn to_halfedge(&self) -> HalfEdgeMesh {
        let mesh = HalfEdgeMesh::new();
        let mut conn = mesh.write_connectivity();
        let mut positions = mesh.write_positions();

        let mut h_idx_to_id = Vec::with_capacity(self.counts.num_halfedges);
        for _ in 0..self.counts.num_halfedges {
            h_idx_to_id.push(conn.alloc_halfedge(HalfEdge::default()));
        }

        let mut v_idx_to_id = Vec::with_capacity(self.counts.num_vertices);
        for v in 0..self.counts.num_vertices {
            v_idx_to_id.push(conn.alloc_vertex(&mut positions, self.vertex_positions[v], None));
        }

        let mut f_idx_to_id = Vec::with_capacity(self.counts.num_faces);
        for _ in 0..self.counts.num_faces {
            f_idx_to_id.push(conn.alloc_face(None));
        }

        // Compute analytical expressions.
        let next_iter = (0..self.counts.num_halfedges).map(|h| self.get_next(h) as u32);
        let face_iter = (0..self.counts.num_halfedges).map(|h| self.get_face(h) as u32);

        for (h, (twin, next, vert, face)) in
            itertools::multizip((&self.twin, next_iter, &self.vert, face_iter)).enumerate()
        {
            let h_id = h_idx_to_id[h];

            let twin_id = twin.map(|idx| h_idx_to_id[idx.get() as usize]);
            let next_id = h_idx_to_id[next as usize];
            let vert_id = v_idx_to_id[*vert as usize];
            let face_id = f_idx_to_id[face as usize];

            conn[h_id] = HalfEdge {
                // If twin id is none, a twin boundary halfedge will be created later
                // in the `add_boundary_halfedges` call.
                twin: twin_id,
                next: Some(next_id),
                vertex: Some(vert_id),
                face: Some(face_id),
            };
            conn[vert_id].halfedge = Some(h_id);
            conn[face_id].halfedge = Some(h_id);
        }

        // The CompactMesh has no boundary halfedges, so we create them here
        conn.add_boundary_halfedges();

        drop(conn);
        drop(positions);
        mesh
    }

    /// Generates the twin pointer for the 4 halfedges spawning from `h` during
    /// subdivision and stores them in `twin[0..4]`.
    fn halfedge_refinement_twin_rule(&self, h: usize, twin: &mut [Option<NonMaxU32>]) {
        // (a) Halfedge's twin rule
        twin[0] = self.twin[h]
            .and_then(|twin_h| NonMaxU32::new(4 * self.get_next(twin_h.get() as usize) as u32 + 3));
        twin[1] = NonMaxU32::new(4 * self.get_next(h) as u32 + 2);
        twin[2] = NonMaxU32::new(4 * self.get_prev(h) as u32 + 1);
        twin[3] = self.twin[self.get_prev(h)]
            .and_then(|twin_prev_h| NonMaxU32::new(4 * twin_prev_h.get()));
    }

    /// Generates the vert pointer for the 4 halfedges spawning from `h` during
    /// subdivision and stores them in `vert[0..4]`
    fn halfedge_refinement_vertex_rule(&self, h: usize, vert: &mut [u32]) {
        let v_d = self.counts.num_vertices as u32;
        let f_d = self.counts.num_faces as u32;

        vert[0] = self.vert[h];
        vert[1] = v_d + f_d + self.edge[h];
        vert[2] = v_d + self.get_face(h) as u32;
        vert[3] = v_d + f_d + self.edge[self.get_prev(h)];
    }

    /// Generates the edge pointer for the 4 halfedges spawning from `h` during
    /// subdivision and stores them in `edge[0..4]`
    fn halfedge_refinement_edge_rule(&self, h: usize, edge: &mut [u32]) {
        let e_d = self.counts.num_edges as u32;
        let h_prev = self.get_prev(h);
        let h_gt_twin_h = self.twin[h]
            .map(|twin_h| (h as u32) < twin_h.get())
            .unwrap_or(true);
        let hp_gt_twin_hp = self.twin[h_prev]
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
            2 * self.edge[h_prev] + 1
        } else {
            2 * self.edge[h_prev]
        };
    }

    /// Returns the next of a given halfedge h. This will use an analytical
    /// expression if the mesh has been subdivided at least once.
    pub fn get_next(&self, h: usize) -> usize {
        if Subdivided {
            if h % 4 == 3 {
                h - 3
            } else {
                h + 1
            }
        } else {
            self.next[h] as usize
        }
    }

    /// Returns the prev of a given halfedge h. This will use an analytical
    /// expression if the mesh has been subdivided at least once.
    pub fn get_prev(&self, h: usize) -> usize {
        if Subdivided {
            if h % 4 == 0 {
                h + 3
            } else {
                h - 1
            }
        } else {
            self.prev[h] as usize
        }
    }

    /// Returns the face of a given halfedge h. This will use an analytical
    /// expression if the mesh has been subdivided at least once.
    pub fn get_face(&self, h: usize) -> usize {
        if Subdivided {
            h / 4
        } else {
            self.face[h] as usize
        }
    }

    /// See "A HalfEdge Refinement Rule for Parallel Catmull-Clark"
    /// https://onrendering.com/data/papers/catmark/HalfedgeCatmullClark.pdf
    ///
    /// If `catmull_clark` is set to true, smooth subdivision using the Catmull
    /// Clark algorithm is performed, otherwise linear subdivision is performed.
    #[profiling::function]
    pub fn subdivide(&self, catmull_clark: bool) -> CompactMesh<true> {
        use rayon::prelude::*;

        // Compute the counts for the new mesh
        let new_counts = self.counts.subdiv();

        // When the mesh has been subdivided at least once, the halfedges will
        // follow a certain structure, allowing some of the computations to be
        // skipped. This is represented by some of the vectors being empty.

        // After subdivision, we have 4 times as many halfedges, exactly.
        let mut new_twin: Vec<Option<NonMaxU32>> = vec![None; new_counts.num_halfedges];
        let mut new_vert = vec![0u32; new_counts.num_halfedges];
        let mut new_edge = vec![0u32; new_counts.num_halfedges];

        // NOTE: We partition the mutable space in the vector into 4-element
        // windows. Window `h` corresponds to halfedges 4h+0..4h+3, using the
        // paper nomenclature

        (
            new_twin.par_chunks_mut(4),
            new_vert.par_chunks_mut(4),
            new_edge.par_chunks_mut(4),
        )
            .into_par_iter()
            .enumerate()
            .for_each(|(h, (twin, vert, edge))| {
                self.halfedge_refinement_twin_rule(h, twin);
                self.halfedge_refinement_vertex_rule(h, vert);
                self.halfedge_refinement_edge_rule(h, edge);
            });

        // The threads need shared access to the vector of atomics, so we have
        // to put them in a vector of atomic floats
        // SAFETY: Vec3 and AtomicVec3 have the exact same memory layout
        let new_vertex_positions =
            unsafe { transmute_vec::<Vec3, AtomicVec3>(vec![Vec3::ZERO; new_counts.num_vertices]) };

        // If the mesh is subdivided, the cycle mesh is 4
        let mut cycle_lengths = Vec::new();
        if !Subdivided {
            (0..self.counts.num_halfedges)
                .into_par_iter()
                .map(|h| {
                    let mut cycle_len = 1;
                    let mut hh = self.get_next(h);
                    while hh != h {
                        cycle_len += 1;
                        hh = self.get_next(hh);
                        if cycle_len > MAX_LOOP_ITERATIONS {
                            break;
                        }
                    }
                    cycle_len as u32
                })
                .collect_into_vec(&mut cycle_lengths);
        }
        let get_cycle_length = move |h: usize| {
            if Subdivided {
                4
            } else {
                cycle_lengths[h]
            }
        };

        let mut valences = Vec::new();
        (0..self.counts.num_halfedges)
            .into_par_iter()
            .map(|h| {
                let mut valence = 1;
                let mut hh = self.get_next(self.twin[h]?.get() as usize);
                while hh != h {
                    valence += 1;
                    hh = self.get_next(self.twin[hh]?.get() as usize);
                    if valence > MAX_LOOP_ITERATIONS {
                        break;
                    }
                }
                NonMaxU32::new(valence as u32)
            })
            .collect_into_vec(&mut valences);

        // --- Face points ---
        (0..self.counts.num_halfedges)
            .into_par_iter()
            .for_each(|h| {
                let m = get_cycle_length(h) as f32;
                let v = self.vert[h] as usize;
                let i = self.counts.num_vertices + self.get_face(h);
                new_vertex_positions[i].fetch_add(
                    self.vertex_positions[v] / m,
                    // NOTE: Relaxed ordering should be okay here. We only care
                    // that this is incremented exactly once per halfedge in the
                    // face, not the order in which threads do it.
                    Ordering::Relaxed,
                );
            });

        // --- Smooth edge points ---
        (0..self.counts.num_halfedges)
            .into_par_iter()
            .for_each(|h| {
                let v = self.vert[h] as usize;
                let i = self.counts.num_vertices + self.get_face(h);
                let j = self.counts.num_vertices + self.counts.num_faces + self.edge[h] as usize;

                // Handle boundary edges as a separate case. During linear
                // subidivision, we simply treat all edges as boundary to apply
                // the simpler rule.
                if self.twin[h].is_some() && catmull_clark {
                    // NOTE: Same rationale as above for relaxed ordering. The
                    // vertices in `i` are not being iterated in this loop, so the
                    // load() does not read a value that changes during this loop
                    let inc = (self.vertex_positions[v]
                        + new_vertex_positions[i].load(Ordering::Relaxed))
                        / 4.0;
                    new_vertex_positions[j].fetch_add(inc, Ordering::Relaxed)
                } else {
                    let v_end = self.vert[self.get_next(h)] as usize;
                    let midpoint = (self.vertex_positions[v] + self.vertex_positions[v_end]) / 2.0;
                    new_vertex_positions[j].store(midpoint, Ordering::Relaxed)
                }
            });

        // --- Smooth vertex points ---
        (0..self.counts.num_halfedges)
            .into_par_iter()
            .for_each(|h| {
                let v = self.vert[h] as usize;
                // If there is a valence, the vertex is not in the boundary.
                // Same as above, the complex rule is only applied for catmull
                // clark subdivision
                if valences[h].is_some() && catmull_clark {
                    let n = valences[h].unwrap().get() as f32;
                    let i = self.counts.num_vertices + self.get_face(h);
                    let j =
                        self.counts.num_vertices + self.counts.num_faces + self.edge[h] as usize;

                    let inc = (4.0 * new_vertex_positions[j].load(Ordering::Relaxed)
                        - new_vertex_positions[i].load(Ordering::Relaxed)
                        + (n - 3.0) * self.vertex_positions[v])
                        / (n * n);

                    new_vertex_positions[v].fetch_add(inc, Ordering::Relaxed);
                } else {
                    new_vertex_positions[v].store(self.vertex_positions[v], Ordering::Relaxed);
                }
            });

        // SAFETY: Same as above, Vec3 and AtomicVec3 have the same memory layout
        let new_vertex_positions =
            unsafe { transmute_vec::<AtomicVec3, Vec3>(new_vertex_positions) };

        CompactMesh {
            twin: new_twin,
            // NOTE: Empty vecs represent analytically computed properties
            prev: vec![],
            next: vec![],
            vert: new_vert,
            edge: new_edge,
            face: vec![],
            vertex_positions: new_vertex_positions,
            counts: new_counts,
        }
    }

    #[profiling::function]
    pub fn subdivide_multi(&self, iterations: usize, catmull_clark: bool) -> CompactMesh<true> {
        let mut mesh = self.subdivide(catmull_clark);
        for _ in 0..(iterations - 1) {
            mesh = mesh.subdivide(catmull_clark);
        }
        mesh
    }
}

/// A counterpart to `glam::Vec3` with atomics in its `x`, `y`, `z` fields.
#[repr(C)]
struct AtomicVec3 {
    pub x: AtomicF32,
    pub y: AtomicF32,
    pub z: AtomicF32,
}

impl AtomicVec3 {
    /// Calls `fetch_add` on each of the inner atomic values internally. Note
    /// that there is one atomic operation per dimension.
    pub fn fetch_add(&self, v: Vec3, order: Ordering) {
        self.x.fetch_add(v.x, order);
        self.y.fetch_add(v.y, order);
        self.z.fetch_add(v.z, order);
    }

    /// Calls `store` on each of the inner atomic values internally. Note
    /// that there is one atomic operation per dimension.
    pub fn store(&self, v: Vec3, order: Ordering) {
        self.x.store(v.x, order);
        self.y.store(v.y, order);
        self.z.store(v.z, order);
    }

    pub fn load(&self, order: Ordering) -> Vec3 {
        Vec3::new(self.x.load(order), self.y.load(order), self.z.load(order))
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
