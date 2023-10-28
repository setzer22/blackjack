// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{
    collections::{BTreeMap, BTreeSet},
    f32::consts::PI,
};

use anyhow::{anyhow, bail};
use float_ord::FloatOrd;
use glam::EulerRot;
use smallvec::SmallVec;

use crate::prelude::*;

use super::selection::SelectionExpression;

/// Just a place where commented-out code goes to die
pub mod deprecated;

/// Removes `h_l` and its twin `h_r`, merging their respective faces together.
/// The face on the L side will be kept, and the R side removed. Both sides of
/// the edge that will be dissolved need to be on a face. Boundary halfedges are
/// not allowed
pub fn dissolve_edge(mesh: &mut MeshConnectivity, h_l: HalfEdgeId) -> Result<()> {
    // --- Collect handles ---
    let h_r = mesh.at_halfedge(h_l).twin().try_end()?;
    // If the face cannot be retrieved, a HalfedgeHasNoFace is returned
    let f_l = mesh.at_halfedge(h_l).face().try_end()?;
    let f_r = mesh.at_halfedge(h_r).face().try_end()?;
    let (v, w) = mesh.at_halfedge(h_l).src_dst_pair().unwrap();

    let h_l_nxt = mesh.at_halfedge(h_l).next().try_end()?;
    let h_l_prv = mesh.at_halfedge(h_l).previous().try_end()?;
    let h_r_nxt = mesh.at_halfedge(h_r).next().try_end()?;
    let h_r_prv = mesh.at_halfedge(h_r).previous().try_end()?;

    let halfedges_r = mesh.halfedge_loop(h_r);

    // --- Fix connectivity ---
    mesh[h_r_prv].next = Some(h_l_nxt);
    mesh[h_l_prv].next = Some(h_r_nxt);
    for h_r in halfedges_r {
        mesh[h_r].face = Some(f_l);
    }
    // Faces or vertices may point to the halfedge we're about to remove. In
    // that case we need to rotate them. We only do it in that case, to avoid
    // modifying the mesh more than necessary.
    if mesh[f_l].halfedge == Some(h_l) {
        mesh[f_l].halfedge = Some(h_l_prv);
    }
    if mesh[v].halfedge == Some(h_l) {
        mesh[v].halfedge = Some(h_l_nxt);
    }
    if mesh[w].halfedge == Some(h_r) {
        mesh[w].halfedge = Some(h_r_nxt);
    }

    // --- Remove elements ---
    mesh.remove_halfedge(h_l);
    mesh.remove_halfedge(h_r);
    mesh.remove_face(f_r);

    Ok(())
}

/// Divides an edge, creating a vertex in between and a new pair of halfedges.
///
/// ## Id Stability
/// Let (v, w) the (src, dst) endpoints of h, and x the new vertex id. It is
/// guaranteed that on the new mesh, the halfedge "h" will remain on the second
/// half of the edge, that is, from x to w. The new edge will go from v to x.
/// Note that this is done in combination with the chamfer operation, whose
/// stability depends on this behavior.
pub fn divide_edge(
    mesh: &mut MeshConnectivity,
    positions: &mut Positions,
    h: HalfEdgeId,
    interpolation_factor: f32,
) -> Result<VertexId> {
    // Select the necessary data elements
    let h_l = h;
    let h_r = mesh.at_halfedge(h_l).twin().try_end()?;
    let h_l_prev = mesh.at_halfedge(h_l).previous().try_end()?;
    let h_r_next = mesh.at_halfedge(h_r).next().try_end()?;
    let f_l = mesh.at_halfedge(h_l).face().try_end().ok();
    let f_r = mesh.at_halfedge(h_r).face().try_end().ok();
    let (v, w) = mesh.at_halfedge(h).src_dst_pair()?;

    // Calculate the new vertex position
    let v_pos = positions[v];
    let w_pos = positions[w];
    let pos = v_pos.lerp(w_pos, interpolation_factor);

    // Allocate new elements
    let x = mesh.alloc_vertex(positions, pos, None);
    let h_l_2 = mesh.alloc_halfedge(HalfEdge::default());
    let h_r_2 = mesh.alloc_halfedge(HalfEdge::default());

    // --- Update connectivity ---

    // Next pointers
    mesh[h_l_2].next = Some(h_l);
    mesh[h_l_prev].next = Some(h_l_2);
    mesh[h_r].next = Some(h_r_2);
    mesh[h_r_2].next = Some(h_r_next);

    // Twin pointers
    mesh[h_l_2].twin = Some(h_r_2);
    mesh[h_r_2].twin = Some(h_l_2);
    mesh[h_l].twin = Some(h_r);
    mesh[h_r].twin = Some(h_l);

    // Vertex pointers
    mesh[h_l].vertex = Some(x);
    mesh[h_r].vertex = Some(w);
    mesh[h_r_2].vertex = Some(x);
    mesh[h_l_2].vertex = Some(v);

    // Face pointers: May be None for boundary
    mesh[h_l_2].face = f_l;
    mesh[h_r_2].face = f_r;

    mesh[x].halfedge = Some(h_l);
    mesh[v].halfedge = Some(h_l_2);

    Ok(x)
}

/// Cuts a face by creating a new edge between vertices `v` and `w`. The
/// vertices must share a face, but not an edge.
pub fn cut_face(
    mesh: &mut halfedge::MeshConnectivity,
    v: VertexId,
    w: VertexId,
) -> Result<HalfEdgeId> {
    let face = mesh
        .at_vertex(v)
        .adjacent_faces()?
        .into_iter()
        .find(|f| mesh.face_vertices(*f).contains(&w))
        .ok_or_else(|| anyhow!("cut_face: v and w must share a face"))?;

    if mesh.at_vertex(v).halfedge_to(w).try_end().is_ok() {
        bail!("cut_face: v and w cannot share an edge")
    }

    let face_halfedges = mesh.face_edges(face);
    if face_halfedges.len() <= 3 {
        bail!("cut_face: cut face only works for quads or higher")
    }

    mesh.add_debug_vertex(v, DebugMark::red("v"));
    mesh.add_debug_vertex(w, DebugMark::red("w"));

    /*
    for h in mesh.at_face(face).halfedges()? {
        mesh.add_debug_halfedge(h, DebugMark::green(""));
    }
    */

    let v_idx = face_halfedges
        .iter()
        .position(|h| mesh.at_halfedge(*h).vertex().end() == v)
        .unwrap() as i32;
    let w_idx = face_halfedges
        .iter()
        .position(|h| mesh.at_halfedge(*h).vertex().end() == w)
        .unwrap() as i32;

    // NOTE: Use rem euclid so negative indices wrap up back at the end
    let h_vprev_v = face_halfedges[(v_idx - 1).rem_euclid(face_halfedges.len() as i32) as usize];
    let h_v_vnext = face_halfedges[v_idx as usize];
    let h_wprev_w = face_halfedges[(w_idx - 1).rem_euclid(face_halfedges.len() as i32) as usize];
    let h_w_wnext = face_halfedges[w_idx as usize];

    // Create new data
    let h_v_w = mesh.alloc_halfedge(HalfEdge::default());
    let h_w_v = mesh.alloc_halfedge(HalfEdge::default());
    let new_face = mesh.alloc_face(None);

    mesh[h_v_w].vertex = Some(v);
    mesh[h_w_v].vertex = Some(w);

    mesh[h_v_w].face = Some(face);
    mesh[h_w_v].face = Some(new_face);

    mesh[h_v_w].twin = Some(h_w_v);
    mesh[h_w_v].twin = Some(h_v_w);

    mesh[h_v_w].next = Some(h_w_wnext);
    mesh[h_w_v].next = Some(h_v_vnext);

    mesh[new_face].halfedge = Some(h_w_v);
    mesh[face].halfedge = Some(h_v_w);

    // Fix connectivity

    mesh[h_vprev_v].next = Some(h_v_w);
    mesh[h_wprev_w].next = Some(h_w_v);

    // The halfedges of the original face that fall on the new face
    let (start, end) = {
        let start = v_idx;
        let mut end = (w_idx - 1).rem_euclid(face_halfedges.len() as i32);
        if end < start {
            end += face_halfedges.len() as i32
        }
        (start, end)
    };
    for i in start..=end {
        let h = face_halfedges[i as usize % face_halfedges.len()];
        mesh[h].face = Some(new_face);
        mesh.add_debug_halfedge(h, DebugMark::blue(""));
    }

    Ok(h_v_w)
}

pub fn dissolve_vertex(mesh: &mut halfedge::MeshConnectivity, v: VertexId) -> Result<FaceId> {
    let outgoing = mesh.at_vertex(v).outgoing_halfedges()?;

    if outgoing.is_empty() {
        bail!("Vertex {:?} is not in a face. Cannot dissolve", v);
    }

    let new_face = mesh.alloc_face(None);

    let mut to_delete = SmallVec::<[_; 16]>::new();

    // Fix next pointers for edges in the new face
    for &h in &outgoing {
        let tw = mesh.at_halfedge(h).twin().try_end()?;
        let w = mesh.at_halfedge(tw).vertex().try_end()?;
        let nxt = mesh.at_halfedge(h).next().try_end()?;
        let prv = mesh.at_halfedge(tw).previous().try_end()?;
        let f = mesh.at_halfedge(h).face().try_end()?;
        mesh[prv].next = Some(nxt);
        if mesh[w].halfedge == Some(tw) {
            mesh[w].halfedge = Some(nxt);
        }

        // We cannot safely remove data at this point, because it could be
        // accessed during `previous()` traversal.
        to_delete.push((tw, h, f));
    }

    // Set all halfedges to the same face
    let outer_loop = mesh.halfedge_loop(mesh.at_halfedge(outgoing[0]).next().try_end()?);
    for &h in &outer_loop {
        mesh[h].face = Some(new_face);
    }
    mesh[new_face].halfedge = Some(outer_loop[0]);

    mesh.remove_vertex(v);
    for (tw, h, f) in to_delete {
        mesh.remove_halfedge(tw);
        mesh.remove_halfedge(h);
        mesh.remove_face(f);
    }

    Ok(new_face)
}

/// Chamfers a vertex. That is, for each outgoing edge of the vertex, a new
/// vertex will be created. All the new vertices will be joined in a new face,
/// and the original vertex will get removed.
///
/// ## Vertices in the boundary
/// When any of the outgoing halfedges for `v` lies in the boundary, this
/// operation cannot be completed as documented, because the final
/// `dissolve_vertex` operation is not well-defined.
///
/// In that case, the operation doesn't complete, and the resulting `FaceId`
/// return value will be `None`. This behavior is not only a best-effort, but is
/// consistent with the expected behavior during the bevel operation, which
/// depends on this operation.
///
/// ## Id Stability
/// This operation guarantees that the outgoing halfedge ids are preserved.
/// Additionally, the returned vertex id vector has the newly created vertex ids
/// provided in the same order as `v`'s outgoing_halfedges
pub fn chamfer_vertex(
    mesh: &mut halfedge::MeshConnectivity,
    positions: &mut Positions,
    v: VertexId,
    interpolation_factor: f32,
) -> Result<(Option<FaceId>, SVec<VertexId>)> {
    let outgoing = mesh.at_vertex(v).outgoing_halfedges()?;
    let mut vertices = SVec::new();
    for &h in &outgoing {
        vertices.push(divide_edge(mesh, positions, h, interpolation_factor)?);
    }

    let mut is_boundary = false;

    for ((&v, _), (&w, &hw)) in vertices
        .iter()
        .zip(outgoing.iter())
        .circular_tuple_windows()
    {
        // Only cut faces at the boundary. If there's two vertices separated by
        // boundary, we take note of that and don't do the final dissolve.
        if !mesh.at_halfedge(hw).is_boundary()? {
            cut_face(mesh, v, w)?;
        } else {
            is_boundary = true;
        }
    }

    if is_boundary {
        Ok((None, vertices))
    } else {
        Ok((Some(dissolve_vertex(mesh, v)?), vertices))
    }
}

/// Creates a 2-sided face on the inside of this edge. This has no effect on the
/// resulting mesh, but it's useful as one of the building blocks of the bevel operation
pub fn duplicate_edge(mesh: &mut MeshConnectivity, h: HalfEdgeId) -> Result<HalfEdgeId> {
    let (v, w) = mesh.at_halfedge(h).src_dst_pair()?;

    let h_v_w = h;
    let h_w_v = mesh.at_halfedge(h).twin().try_end()?;

    let h2_v_w = mesh.alloc_halfedge(HalfEdge::default());
    let h2_w_v = mesh.alloc_halfedge(HalfEdge::default());

    let inner_face = mesh.alloc_face(Some(h2_v_w));

    // The two new halfedges make a cycle (2-sided face)
    mesh[h2_v_w].face = Some(inner_face);
    mesh[h2_w_v].face = Some(inner_face);
    mesh[h2_v_w].next = Some(h2_w_v);
    mesh[h2_w_v].next = Some(h2_v_w);

    mesh[h2_v_w].vertex = Some(v);
    mesh[h2_w_v].vertex = Some(w);

    // The twins point to the respective outer halfedges of the original edge
    mesh[h2_v_w].twin = Some(h_w_v);
    mesh[h2_w_v].twin = Some(h_v_w);
    mesh[h_w_v].twin = Some(h2_v_w);
    mesh[h_v_w].twin = Some(h2_w_v);

    Ok(h2_v_w)
}

/// Merges the src and dst vertices of `h` so that only the first one remains
pub fn collapse_edge(mesh: &mut MeshConnectivity, h: HalfEdgeId) -> Result<VertexId> {
    let (v, w) = mesh.at_halfedge(h).src_dst_pair()?;
    let t = mesh.at_halfedge(h).twin().try_end()?;
    let h_next = mesh.at_halfedge(h).next().try_end()?;
    let h_prev = mesh.at_halfedge(h).previous().try_end()?;
    let t_next = mesh.at_halfedge(t).next().try_end()?;
    let t_prev = mesh.at_halfedge(t).previous().try_end()?;
    let w_outgoing = mesh.at_vertex(w).outgoing_halfedges()?;
    let f_h = mesh.at_halfedge(h).face_or_boundary()?;
    let f_t = mesh.at_halfedge(t).face_or_boundary()?;
    // We check here if either face is a triangle. This is an edge case that
    // requires some additional post-processing later.
    let f_h_is_triangle = f_h.is_some() && mesh.halfedge_loop_iter(h).count() == 3;
    let f_t_is_triangle = f_t.is_some() && mesh.halfedge_loop_iter(t).count() == 3;

    // --- Adjust connectivity ---
    for h_wo in w_outgoing {
        mesh[h_wo].vertex = Some(v);
    }
    mesh[t_prev].next = Some(t_next);
    mesh[h_prev].next = Some(h_next);

    // Some face may point to the halfedges we're deleting. Fix that.
    if let Some(f_h) = f_h {
        if mesh.at_face(f_h).halfedge().try_end()? == h {
            mesh[f_h].halfedge = Some(h_next);
        }
    }
    if let Some(f_t) = f_t {
        if mesh.at_face(f_t).halfedge().try_end()? == t {
            mesh[f_t].halfedge = Some(t_next);
        }
    }

    // --- Remove data ----
    mesh.remove_halfedge(t);
    mesh.remove_halfedge(h);
    mesh.remove_vertex(w);

    // --- Triangular face post-processing ---

    // If either f_h or f_t were triangle faces, we need to do some extra
    // cleanup, because the collapse edge operation also removes those faces.

    /// The operation returns a pair of halfedges, which are the external edges
    /// of the triangular face after the internal ones have been deleted. After
    /// this operation, the triangular face is now a single edge.
    fn post_process_triangular_face(
        mesh: &mut MeshConnectivity,
        prev: HalfEdgeId,
        next: HalfEdgeId,
        face: Option<FaceId>,
    ) -> Result<(HalfEdgeId, HalfEdgeId)> {
        let prev_twin = mesh.at_halfedge(prev).twin().try_end()?;
        let next_twin = mesh.at_halfedge(next).twin().try_end()?;
        mesh[prev_twin].twin = Some(next_twin);
        mesh[next_twin].twin = Some(prev_twin);
        mesh.remove_halfedge(prev);
        mesh.remove_halfedge(next);
        if let Some(face) = face {
            mesh.remove_face(face);
        }
        Ok((prev_twin, next_twin))
    }

    let f_h_triangle_halfedges = if f_h_is_triangle {
        Some(post_process_triangular_face(mesh, h_prev, h_next, f_h)?)
    } else {
        None
    };
    let f_t_triangle_halfedges = if f_t_is_triangle {
        Some(post_process_triangular_face(mesh, t_prev, t_next, f_t)?)
    } else {
        None
    };

    // --- Fix connectivity for vertices ---

    // The remaining vertices may be pointing to a deleted halfedge. We need to
    // fix that here to prevent consistency issues.
    if mesh[v].halfedge == Some(h) {
        // In general, we can use `h_next` since that is not an outgoing
        // halfedge of `v (because `h` was collapsed). But in case `f_h` was a
        // triangle we need to use `h_v_x` since `h_next` was deleted.
        if let Some((h_v_x, _)) = f_h_triangle_halfedges {
            mesh[v].halfedge = Some(h_v_x);
        } else {
            mesh[v].halfedge = Some(h_next);
        }
    }
    if let Some((_, h_x_w)) = f_h_triangle_halfedges {
        let x = mesh.at_halfedge(h_x_w).vertex().try_end()?;
        if mesh[x].halfedge == Some(h_prev) {
            mesh[x].halfedge = Some(h_x_w);
        }
    }
    if let Some((h_v_y, h_y_v)) = f_t_triangle_halfedges {
        let y = mesh.at_halfedge(h_y_v).vertex().try_end()?;
        if mesh[y].halfedge == Some(t_prev) {
            mesh[y].halfedge = Some(h_y_v);
        }

        if mesh[v].halfedge == Some(t_next) {
            mesh[v].halfedge = Some(h_v_y);
        }
    }

    Ok(v)
}

/// Adjusts the connectivity of the mesh in preparation for a bevel operation.
/// Any `halfedges` passed in will get "duplicated", and a face will be created
/// in-between, consistently adjusting the connectivity everywhere.
///
/// # Returns
/// A set of halfedges that participated in the bevel. These are the halfedges
/// that touched any of the original faces of the mesh. Thus, it is guaranteed
/// that any of their twins is touching a newly created face.
fn bevel_edges_connectivity(
    mesh: &mut MeshConnectivity,
    positions: &mut Positions,
    halfedges: &[HalfEdgeId],
) -> Result<BTreeSet<HalfEdgeId>> {
    let mut edges_to_bevel = BTreeSet::new();
    let mut duplicated_edges = BTreeSet::new();
    let mut vertices_to_chamfer = BTreeSet::new();

    // ---- 1. Duplicate all edges -----
    for &h in halfedges {
        // NOTE: Ignore edges for which we already handled its twin
        let not_yet_handled =
            edges_to_bevel.insert(h) && edges_to_bevel.insert(mesh[h].twin.unwrap());
        if not_yet_handled {
            let h_dup = duplicate_edge(mesh, h)?;
            duplicated_edges.insert(h_dup);
            duplicated_edges.insert(mesh.at_halfedge(h_dup).next().try_end()?);
            let (src, dst) = mesh.at_halfedge(h).src_dst_pair()?;
            vertices_to_chamfer.insert(src);
            vertices_to_chamfer.insert(dst);
        }
    }

    // ---- 2. Chamfer all vertices -----

    // There are two kinds of edge collapse operations, depending on wether the
    // chamfer operation can cut the face between these two vertices. Sometimes
    // faces can't be cut because the two vertices are separated by the boundary.

    /// This is the regular operation, where a cut operation was performed
    /// between two vertices, and we now want to collapse this cut edge.
    struct CollapseAcrossFace {
        x: VertexId,
        y: VertexId,
    }
    /// This is the special case, where the cut operation couldn't be performed.
    /// In that case, instead of collapsing the two vertices, we collapse them
    /// into the central one (the one we chamfered).
    struct CollapseAcrossBoundary {
        x: VertexId,
        y: VertexId,
        central: VertexId,
    }

    // "Collapse across boundary" ops are deferred. If we do them locally for
    // each vertex during chamfer we may end up introducing inconsistencies.
    let mut deferred_collapse_ops: Vec<CollapseAcrossBoundary> = vec![];

    // Since we're freely collapsing vertices, the reified operations may
    // contain references to vertices that no longer exist. This translation map
    // is used to know where vertices end up and avoid accessing invalid ids.
    type TranslationMap = HashMap<VertexId, VertexId>;
    let mut translation_map: TranslationMap = HashMap::new();

    /// Returns the translation of a vertex, that is, the vertex this vertex
    /// ended up being translated to.
    fn get_translated(m: &TranslationMap, v: VertexId) -> VertexId {
        let mut v = v;
        // Translations may be transitive.. chase until we reach a vertex that
        // has no translation, this is the one that still exists.
        while let Some(v_tr) = m.get(&v) {
            v = *v_tr;
        }
        v
    }

    for central_vertex in vertices_to_chamfer {
        let outgoing_halfedges = mesh.at_vertex(central_vertex).outgoing_halfedges()?;

        // After the chamfer operation, some vertex pairs need to get collapsed
        // into a single one. The meaning of 'collapse' depends on whether the
        // vertices are joined by an edge, or separated by the boundary.
        //
        // This binary vector has a `true` for every vertex position where that
        // needs to happen.
        let num_hs_to_bevel: u32 = outgoing_halfedges
            .iter()
            .map(|h| edges_to_bevel.contains(h) as u32)
            .sum();
        let collapse_indices = outgoing_halfedges
            .iter()
            .circular_tuple_windows()
            .map(|(h, h2)| {
                let h_b = edges_to_bevel.contains(h);
                let h2_b = edges_to_bevel.contains(h2);
                let h_d = duplicated_edges.contains(h);
                let h2_d = duplicated_edges.contains(h2);
                let h_n = !h_b && !h_d;
                let h2_n = !h2_b && !h2_d;

                h_b && h2_n
                    || h_d && h2_b
                    || h_d && h2_n
                    || h_n && h2_b
                    // NOTE: When we have exactly two edges to bevel in this
                    // vertex, doing this gives nicer results (and is more
                    // consistent with other 3d apps like Blender).
                    || if num_hs_to_bevel == 2 {
                        h_n && h2_n
                    } else {
                        false
                    }
            })
            .collect::<SVecN<_, 16>>();

        // Here, we execute the chamfer operation. The returned indices are
        // guaranteed to be in the same order as `v`'s outgoing halfedges.
        let (_, new_verts) = chamfer_vertex(mesh, positions, central_vertex, 0.0)?;

        let mut local_collapse_ops: Vec<CollapseAcrossFace> = vec![];

        for ((&x, &y), should_collapse) in new_verts
            .iter()
            .circular_tuple_windows()
            .zip(collapse_indices)
        {
            if should_collapse {
                let shared_face = mesh.at_vertex(x).adjacent_faces().ok().and_then(|faces| {
                    faces
                        .into_iter()
                        .find(|f| mesh.face_vertices(*f).contains(&y))
                });

                // When the shared face between y and x is the boundary, we
                // can't collapse the edge between the two because it
                // doesn't exist. The correct fix here is to collapse both
                // vertices into the central one. The chamfer operation will
                // keep the central vertex if at least one of its adjacent
                // faces was the boundary.
                if shared_face.is_some() {
                    local_collapse_ops.push(CollapseAcrossFace { x, y })
                } else {
                    deferred_collapse_ops.push(CollapseAcrossBoundary {
                        x,
                        y,
                        central: central_vertex,
                    })
                }
            }
        }

        for CollapseAcrossFace { x, y } in local_collapse_ops {
            // Collapse the shared edge between the vertices
            let x = get_translated(&translation_map, x);
            let y = get_translated(&translation_map, y);
            let h = mesh.at_vertex(y).halfedge_to(x).try_end()?;
            collapse_edge(mesh, h)?;
            translation_map.insert(x, y); // Take note that x is now y
        }
    }

    for CollapseAcrossBoundary { x, y, central } in deferred_collapse_ops {
        // Collapse both vertices into the central one
        let x = get_translated(&translation_map, x);
        let y = get_translated(&translation_map, y);
        let central_vertex = get_translated(&translation_map, central);

        let h1 = mesh.at_vertex(central_vertex).halfedge_to(x).try_end()?;
        collapse_edge(mesh, h1)?;

        let h2 = mesh.at_vertex(central_vertex).halfedge_to(y).try_end()?;
        collapse_edge(mesh, h2)?;

        translation_map.insert(x, central_vertex); // Take note of the change
        translation_map.insert(y, central_vertex); // Take note of the change
    }

    Ok(edges_to_bevel)
}

/// Bevels the given vertices by a given distance amount
pub fn bevel_edges(
    mesh: &mut MeshConnectivity,
    positions: &mut Positions,
    halfedges: &[HalfEdgeId],
    amount: f32,
) -> Result<()> {
    let beveled_edges = bevel_edges_connectivity(mesh, positions, halfedges)?;

    // --- Adjust vertex positions ---

    // Movement of vertices in a bevel can be modelled as a set of pulls. For
    // each beveled edge in which the vertex participates, a certain "pull" will
    // be exerted in the direction of either the next, or previous edge
    // depending on their location of the halfedge (head, tail resp.). The final
    // move direction of a vertice is the sum of all its pulls.
    let mut move_ops = HashMap::<VertexId, HashSet<Vec3Ord>>::new();
    for h in beveled_edges {
        mesh.add_debug_halfedge(h, DebugMark::green("bvl"));

        if mesh.at_halfedge(h).is_boundary()? {
            continue;
        }

        let (v, w) = mesh.at_halfedge(h).src_dst_pair()?;
        let v_to = mesh.at_halfedge(h).previous().vertex().try_end()?;
        let v_to_pos = positions[v_to];
        let w_to = mesh.at_halfedge(h).next().next().vertex().try_end()?;
        let w_to_pos = positions[w_to];

        let vdir = move_ops.entry(v).or_insert_with(HashSet::new);
        vdir.insert(v_to_pos.to_ord());

        let wdir = move_ops.entry(w).or_insert_with(HashSet::new);
        wdir.insert(w_to_pos.to_ord());
    }

    for (v, v_pulls) in move_ops {
        let v_pos = positions[v];
        for v_pull in v_pulls {
            let pull_to = v_pull.to_vec();
            let dir = (pull_to - v_pos).normalize();
            positions[v] += dir * amount;
        }
    }

    Ok(())
}

/// Extrudes the given set of faces. Faces that are connected by at least one
/// edge will be connected after the extrude.
pub fn extrude_faces(
    mesh: &mut MeshConnectivity,
    positions: &mut Positions,
    faces: &[FaceId],
    amount: f32,
) -> Result<()> {
    let face_set: HashSet<FaceId> = faces.iter().cloned().collect();

    // Find the set of all halfedges not adjacent to another extruded face.
    let mut halfedges = vec![];
    for f in faces {
        for h in mesh.at_face(*f).halfedges()? {
            let twin = mesh.at_halfedge(h).twin().try_end()?;
            if let Ok(tw_face) = mesh.at_halfedge(twin).face().try_end() {
                if !face_set.contains(&tw_face) {
                    halfedges.push(h);
                }
            } else {
                halfedges.push(h);
            }
        }
    }

    let _beveled_edges = bevel_edges_connectivity(mesh, positions, &halfedges)?;

    // --- Adjust vertex positions ---

    // For each face, each vertex is pushed in the direction of the face's
    // normal vector. Vertices that share more than one face, get accumulated
    // pushes.
    let mut move_ops = HashMap::<VertexId, HashSet<Vec3Ord>>::new();

    for face in faces {
        for v in mesh.at_face(*face).vertices()? {
            let push = mesh
                .face_normal(positions, *face)
                .ok_or_else(|| anyhow!("Attempted to extrude a face with only two vertices."))?;
            move_ops.entry(v).or_default().insert(push.to_ord());
        }
    }

    for (v_id, ops) in move_ops {
        positions[v_id] += ops
            .iter()
            .fold(Vec3::ZERO, |x, y| x + y.to_vec())
            .normalize()
            * amount;
    }

    Ok(())
}

/// Generates the flat normals channel for this mesh
pub fn generate_flat_normals_channel(mesh: &HalfEdgeMesh) -> Result<Channel<FaceId, Vec3>> {
    let positions = mesh.read_positions();
    let conn = mesh.read_connectivity();
    let mut normals = Channel::<FaceId, Vec3>::new();

    for (face, _) in conn.iter_faces() {
        // NOTE: Faces with only 2 vertices get a zero normal.
        normals[face] = conn.face_normal(&positions, face).unwrap_or(Vec3::ZERO);
    }

    Ok(normals)
}

/// Computes the flat normal channel for this mesh and configures the mesh to
/// generate flat normals. Flat normals are attached to faces.
pub fn set_flat_normals(mesh: &mut HalfEdgeMesh) -> Result<()> {
    let normals = generate_flat_normals_channel(mesh)?;
    let normals_ch_id = mesh
        .channels
        .replace_or_create_channel("face_normal", normals);

    mesh.default_channels.face_normals = Some(normals_ch_id);
    mesh.gen_config.smooth_normals = false;

    Ok(())
}

/// Generates the smooth normals channel for this mesh.
pub fn generate_smooth_normals_channel(mesh: &HalfEdgeMesh) -> Result<Channel<VertexId, Vec3>> {
    let positions = mesh.read_positions();
    let conn = mesh.read_connectivity();
    let mut normals = Channel::<VertexId, Vec3>::new();

    for (vertex, _) in conn.iter_vertices() {
        let adjacent_faces = conn.at_vertex(vertex).adjacent_faces()?;
        let mut normal = Vec3::ZERO;
        for face in adjacent_faces.iter_cpy() {
            normal += conn.face_normal(&positions, face).unwrap_or(Vec3::ZERO);
        }
        normals[vertex] = normal.normalize_or_zero();
    }

    Ok(normals)
}

/// Computes "flat" normals for this mesh. Flat normals are attached to faces.
pub fn set_smooth_normals(mesh: &mut HalfEdgeMesh) -> Result<()> {
    let normals = generate_smooth_normals_channel(mesh)?;
    let normals_ch_id = mesh
        .channels
        .replace_or_create_channel("vertex_normal", normals);

    mesh.gen_config.smooth_normals = true;
    mesh.default_channels.vertex_normals = Some(normals_ch_id);

    Ok(())
}

/// Generates an UV channel for the mesh where ever polygon is mapped to the
/// full UV range. Triangles will take half the UV space, quads will take the
/// full space, and n-gons will take as much space as possible, being centered
/// in the middle.
pub fn generate_full_range_uvs_channel(mesh: &HalfEdgeMesh) -> Result<Channel<HalfEdgeId, Vec3>> {
    let conn = mesh.read_connectivity();
    let mut uvs = Channel::<HalfEdgeId, Vec3>::new();

    for (face, _) in conn.iter_faces() {
        // We use halfedges as a proxy for vertices, because we are interested
        // in vertices, not just as points in space, but we actually want
        // separate vertices for each face.
        let halfedges = conn.face_edges(face);
        match halfedges.len() {
            x if x <= 2 => { /* Ignore */ }
            3 => {
                // Triangle
                uvs[halfedges[0]] = Vec3::new(1.0, 0.0, 0.0);
                uvs[halfedges[1]] = Vec3::new(1.0, 1.0, 0.0);
                uvs[halfedges[2]] = Vec3::new(0.0, 1.0, 0.0);
            }
            4 => {
                // Quad
                uvs[halfedges[0]] = Vec3::new(0.0, 0.0, 0.0);
                uvs[halfedges[1]] = Vec3::new(1.0, 0.0, 0.0);
                uvs[halfedges[2]] = Vec3::new(1.0, 1.0, 0.0);
                uvs[halfedges[3]] = Vec3::new(0.0, 1.0, 0.0);
            }
            len => {
                // N-gon
                let angle_delta = 2.0 * PI / len as f32;
                for i in 0..len {
                    let q = Quat::from_rotation_y(angle_delta * i as f32);
                    uvs[halfedges[i]] = Vec3::ONE * 0.5 + (q * Vec3::Y);
                }
            }
        }
    }

    Ok(uvs)
}

pub fn set_full_range_uvs(mesh: &mut HalfEdgeMesh) -> Result<()> {
    let uvs = generate_full_range_uvs_channel(mesh)?;
    let uvs_ch_id = mesh.channels.replace_or_create_channel("uv", uvs);
    mesh.default_channels.uvs = Some(uvs_ch_id);
    Ok(())
}

pub fn make_quad(conn: &mut MeshConnectivity, verts: &[VertexId]) -> Result<()> {
    if verts.len() != 4 {
        bail!("The make_quad operation only accepts quads.")
    }

    #[derive(Clone, Copy, Debug, Default)]
    struct EdgeInfo {
        /// The id of the halfedge
        id: HalfEdgeId,
        /// Did the halfedge exist in the original mesh?
        existed: bool,
    }

    // The new quad face
    let face = conn.alloc_face(None);

    // The halfedges in the interior loop, the one that will hold the quad
    // - NOTE: Default data is replaced in the loop
    let mut a_edges = [EdgeInfo::default(); 4];
    // The halfedges in the exterior loop, the twins of interior_hs, in the same
    // order, so their next pointers are reversed to the order of the array.
    let mut b_edges = [EdgeInfo::default(); 4];

    // Fill the arrays
    for (i, (v1, v2)) in verts.iter_cpy().circular_tuple_windows().enumerate() {
        let a_i = conn.at_vertex(v1).halfedge_to(v2).try_end().ok();
        let b_i = conn.at_vertex(v2).halfedge_to(v1).try_end().ok();

        // Take note of any existing arcs. Generate new halfedges otherwise. We
        // will tie them up later.
        a_edges[i] = EdgeInfo {
            id: a_i.unwrap_or_else(|| conn.alloc_halfedge(HalfEdge::default())),
            existed: a_i.is_some(),
        };
        b_edges[i] = EdgeInfo {
            id: b_i.unwrap_or_else(|| conn.alloc_halfedge(HalfEdge::default())),
            existed: b_i.is_some(),
        };
    }

    // If any of the inner edges already has a face, we can't make the quad.
    for e in a_edges.iter() {
        if !conn.at_halfedge(e.id).is_boundary()? {
            bail!(
                "All halfedges must be in boundary to make a quad but {:?} isn't",
                e.id
            )
        }
    }

    fn prev_i(i: usize, n: usize) -> usize {
        // NOTE: Use rem_euclid for correct negative modulus and cast to isize
        // to avoid underflow.
        ((i as isize - 1).rem_euclid(n as isize)) as usize
    }

    // Compute the predecessors of a in the original graph. We can only do this
    // as long as the mesh is well-formed because the `previous()` operator
    // traverses a full halfedge loop.
    let mut a_prev_orig = [Default::default(); 4];
    for (i, a_i) in a_edges.iter_cpy().enumerate() {
        if a_i.existed {
            a_prev_orig[i] = conn.at_halfedge(a_i.id).previous().try_end()?;
        }
    }

    // Fix the next pointer for 'a' predecessors (if any)
    for (i, a_i) in a_edges.iter_cpy().enumerate() {
        if a_i.existed {
            conn[a_prev_orig[i]].next = Some(b_edges[prev_i(i, 4)].id);
        }
    }

    // Fill data for the 'b' halfedges.
    for (i, b_i) in b_edges.iter_cpy().enumerate() {
        conn[b_i.id].twin = Some(a_edges[i].id);
        conn[b_i.id].vertex = Some(verts[(i + 1) % 4]);
        conn[b_i.id].next = if b_i.existed {
            conn[b_i.id].next
        } else {
            let a_prev = a_edges[prev_i(i, 4)];
            if a_prev.existed {
                Some(
                    conn[a_prev.id]
                        .next
                        .ok_or_else(|| anyhow!("Fatal: Halfedge should have next"))?,
                )
            } else {
                Some(b_edges[prev_i(i, 4)].id)
            }
        };
        conn[b_i.id].face = if b_i.existed {
            conn[b_i.id].face
        } else {
            None // None here means boundary
        }
    }

    // Fill data for the 'a' halfedges. This happens last because we need some
    // data from the original connectivity before we override it.
    for (i, a_i) in a_edges.iter_cpy().enumerate() {
        conn[a_i.id].next = Some(a_edges[(i + 1) % 4].id);
        conn[a_i.id].twin = Some(b_edges[i].id);
        conn[a_i.id].face = Some(face);
        conn[a_i.id].vertex = Some(verts[i]);
    }

    // Give the face a halfedge
    conn[face].halfedge = Some(a_edges[0].id);

    // For verts that were disconnected, give them a halfedge
    for (i, v) in verts.iter_cpy().enumerate() {
        conn[v].halfedge = Some(a_edges[i].id)
    }

    Ok(())
}

/// Connects two (not necessarily closed) edge chains with faces. Edges are
/// implicitly defined by the 2-size windows of vertices.
pub fn bridge_chains(
    mesh: &mut HalfEdgeMesh,
    chain_1: &[VertexId],
    chain_2: &[VertexId],
    is_closed: bool,
) -> Result<()> {
    if chain_1.len() != chain_2.len() {
        bail!("Loops to bridge need to be of the same length.")
    }
    if chain_1.is_empty() || chain_2.is_empty() {
        bail!("Loops to bridge cannot be empty.")
    }

    let mut conn = mesh.write_connectivity();
    let positions = mesh.read_positions();
    let chain_len = chain_1.len(); // same length

    for (v, w) in chain_1
        .iter()
        .tuple_windows()
        .chain(chain_2.iter().tuple_windows())
    {
        if !conn.at_vertex(*v).halfedge_to(*w).is_boundary()? {
            bail!("Cannot bridge loops with edges that are not in a boundary. This would lead to a non-manifold mesh.");
        }
    }

    for v in chain_1.iter_cpy() {
        if chain_2.contains(&v) {
            bail!("Trying to bridge the same loop.")
        }
    }

    // Each vertex in the first loop needs to be mapped to a vertex in the other
    // loop. When the loops are open, there's just a single way to do it, but
    // when the loops are closed there's `loop_len` possible combinations. We
    // find the best possible mapping which minimizes the sum of distances
    // between vertex pairs
    let chain_1_best_shift = if is_closed {
        // Computes the sum of distances after shifting verts_1 by i positions
        let sum_distances_rotated = |i: usize| {
            let x = FloatOrd(
                rotate_iter(chain_1.iter_cpy(), i, chain_len)
                    .enumerate()
                    .map(|(j, v_sh)| {
                        // NOTE: We index verts_2 backwards with respect to
                        // verts_1. This is because the two chains are facing in
                        // opposite directions, otherwise we wouldn't be able to
                        // bridge them
                        positions[v_sh].distance_squared(positions[chain_2[(chain_len - 1) - j]])
                    })
                    .sum::<f32>(),
            );
            x
        };

        // We memoize the sum_distances in a vec because it's a relatively
        // expensive function and `position_min_by_key` will call it multiple
        // times per key.
        let distances = (0..chain_len).map(sum_distances_rotated).collect_vec();

        (0..chain_len)
            .position_min_by_key(|i| distances[*i])
            .expect("Loop should not be empty.")
    } else {
        // The no-op rotation, in case of bridging two open loops.
        0
    };

    let chain_1_shifted =
        rotate_iter(chain_1.iter_cpy(), chain_1_best_shift, chain_len).collect_vec();

    for (i, ((v1, v2), (v3, v4))) in chain_1_shifted
        .iter_cpy()
        .branch(
            is_closed,
            |it| it.circular_tuple_windows(),
            |it| it.tuple_windows(),
        )
        .zip(chain_2.iter_cpy().rev().branch(
            is_closed,
            |it| it.circular_tuple_windows(),
            |it| it.tuple_windows(),
        ))
        .enumerate()
    {
        conn.add_debug_vertex(v1, DebugMark::blue(&format!("{i}",)));
        conn.add_debug_vertex(v3, DebugMark::blue(&format!("{i}",)));
        make_quad(&mut conn, &[v1, v2, v4, v3])?;
    }

    Ok(())
}

pub fn sort_bag_of_edges(
    mesh: &MeshConnectivity,
    bag: &[HalfEdgeId],
) -> Result<(SVec<VertexId>, bool)> {
    /// An ordered pair of halfedges
    #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
    pub struct EdgeId {
        a: HalfEdgeId,
        b: HalfEdgeId,
    }

    impl EdgeId {
        pub fn new(h1: HalfEdgeId, h2: HalfEdgeId) -> Self {
            assert!(
                h1 != h2,
                "Invariant: Don't create an EdgeId for two equal halfedges."
            );
            Self {
                a: h1.min(h2),
                b: h1.max(h2),
            }
        }

        pub fn find_other(&self, conn: &MeshConnectivity, v: VertexId) -> VertexId {
            let (src, dst) = conn.at_halfedge(self.a).src_dst_pair().unwrap();
            if v == src {
                dst
            } else {
                src
            }
        }
    }

    if bag.is_empty() {
        bail!("Bag cannot be empty");
    }

    // Stores a mapping between vertices and the edges they participate in.
    let mut vert_to_edges = BTreeMap::<VertexId, BTreeSet<EdgeId>>::new();

    for h in bag.iter_cpy() {
        let (src, dst) = mesh.at_halfedge(h).src_dst_pair()?;
        let twin = mesh.at_halfedge(h).twin().try_end()?;
        let edge_id = EdgeId::new(h, twin);
        vert_to_edges.entry(src).or_default().insert(edge_id);
        vert_to_edges.entry(dst).or_default().insert(edge_id);
    }

    let endpoints = vert_to_edges
        .iter()
        .filter(|(_, es)| es.len() == 1)
        .map(|(v, _)| *v)
        .collect_svec();

    if endpoints.is_empty() {
        // If there are no endpoints, it means the edges form a closed loop.
        // (Or more than one, this gets checked later on.)

        // If the halfedges have a loop, we simply break the loop and
        // restart the function.
        let e = vert_to_edges
            .iter_mut()
            .next()
            .and_then(|(_, es)| es.pop_first2())
            .expect("Not empty");
        let new_bag = bag
            .iter_cpy()
            .filter(|h| e.a != *h && e.b != *h)
            .collect_vec();
        let (verts, _) = sort_bag_of_edges(mesh, &new_bag)?;
        Ok((verts, true)) // Mark the loop
    } else {
        // We take the first endpoint. To get the other loop, reverse list.
        let endpoint = endpoints[0];
        let mut sorted_vertices = SVec::new();

        let mut v = endpoint;
        while sorted_vertices.len() < vert_to_edges.len() {
            if sorted_vertices.contains(&v) {
                bail!("Halfedges do not form a chain.")
            }

            let v_es = vert_to_edges.get_mut(&v).unwrap();
            if v_es.len() == 1 {
                let v_e = v_es.pop_first2().unwrap();
                let w = v_e.find_other(mesh, v);

                // Remove the edge from the other vertex, now it is an endpoint.
                let w_es = vert_to_edges.get_mut(&w).unwrap();
                w_es.remove(&v_e);

                sorted_vertices.push(v);
                v = w;
            } else if v_es.is_empty() {
                sorted_vertices.push(v);
                break;
            } else {
                bail!("Halfedges do not form a chain")
            }
        }

        Ok((sorted_vertices, false))
    }
}

/// Same as `bridge_chains`, but a bit smarter. Instead of taking the two
/// ordered chains, it takes two bags of edges that come from a UI selection.
/// sorts them and figures out the right order before calling `bridge_chains`.
/// This is helpful when the set of edges was obtained as a manual selection
/// from the UI.
///
/// The extra flip parameter lets you select all permutations of flipping either
/// the first or second chain, leading to different winding orders.
pub fn bridge_chains_ui(
    mesh: &mut HalfEdgeMesh,
    bag_1: &[HalfEdgeId],
    bag_2: &[HalfEdgeId],
    flip: usize,
) -> Result<()> {
    if bag_1.is_empty() || bag_2.is_empty() {
        bail!("Loops cannot be empty")
    }

    let conn = mesh.write_connectivity();
    let (mut chain_1, is_closed_1) = sort_bag_of_edges(&conn, bag_1)?;
    let (mut chain_2, is_closed_2) = sort_bag_of_edges(&conn, bag_2)?;
    drop(conn);

    if is_closed_1 != is_closed_2 {
        bail!("You can't bridge a closed chain with an open chain.")
    }
    let is_closed = is_closed_1;

    match (flip + 1) % 4 {
        // That +1 is experimentally determined to give nice results
        0 => {}
        1 => {
            chain_1.reverse();
        }
        2 => {
            chain_2.reverse();
        }
        3 => {
            chain_1.reverse();
            chain_2.reverse();
        }
        _ => unreachable!(),
    }

    bridge_chains(mesh, &chain_1, &chain_2, is_closed)?;

    Ok(())
}

pub fn transform(mesh: &HalfEdgeMesh, translate: Vec3, rotate: Vec3, scale: Vec3) -> Result<()> {
    let mut positions = mesh.write_positions();
    let conn = mesh.read_connectivity();

    for (v, _) in conn.iter_vertices() {
        positions[v] = Quat::from_euler(glam::EulerRot::XYZ, rotate.x, rotate.y, rotate.z)
            * (positions[v] * scale)
            + translate;
    }

    Ok(())
}

/// Creates a new bool channel with the given `group_name`. The group will
/// contain all the elements matching `selection` for the given type of mesh
/// element `kt`.
///
/// Returns an error if a group with the same name already exists.
pub fn make_group(
    mesh: &mut HalfEdgeMesh,
    kt: ChannelKeyType,
    selection: &SelectionExpression,
    group_name: &str,
) -> Result<()> {
    macro_rules! impl_branch {
        ($channel_type:ty, $resolve_fn:ident) => {{
            let ch_id = mesh
                .channels
                .create_channel::<$channel_type, bool>(group_name)?;
            let mut group_ch = mesh.channels.write_channel(ch_id)?;
            let ids = mesh.$resolve_fn(selection)?;
            // Channel's default is false, we only need to set the true keys.
            for id in ids {
                group_ch[id] = true;
            }
        }};
    }

    match kt {
        ChannelKeyType::VertexId => {
            impl_branch! { VertexId, resolve_vertex_selection_full }
        }
        ChannelKeyType::FaceId => {
            impl_branch! { FaceId, resolve_face_selection_full }
        }
        ChannelKeyType::HalfEdgeId => {
            impl_branch! { HalfEdgeId, resolve_halfedge_selection_full }
        }
    }

    Ok(())
}

/// Adds a disconnected edge to the mesh
pub fn add_edge(mesh: &HalfEdgeMesh, start: Vec3, end: Vec3) -> Result<(HalfEdgeId, HalfEdgeId)> {
    let mut conn = mesh.write_connectivity();
    let mut positions = mesh.write_positions();

    let v_src = conn.alloc_vertex(&mut positions, start, None);
    let v_dst = conn.alloc_vertex(&mut positions, end, None);

    let h_src = conn.alloc_halfedge(HalfEdge::default());
    let h_dst = conn.alloc_halfedge(HalfEdge::default());

    conn[v_src].halfedge = Some(h_src);
    conn[v_dst].halfedge = Some(h_dst);

    conn[h_src].next = Some(h_dst);
    conn[h_src].twin = Some(h_dst);
    conn[h_src].vertex = Some(v_src);
    conn[h_src].face = None;

    conn[h_dst].next = Some(h_src);
    conn[h_dst].twin = Some(h_src);
    conn[h_dst].vertex = Some(v_dst);
    conn[h_dst].face = None;

    Ok((h_src, h_dst))
}

/// Creates a new edge from an existing edge and a new edge, that will be placed
/// at the given position. The VertexId for the new edge is returned.
///
/// This is an internal operations and assumes the given vertex is at the tip of
/// a curve. It is used to incrementally construct polylines.
fn add_edge_chain(mesh: &HalfEdgeMesh, start: VertexId, end: Vec3) -> Result<VertexId> {
    let mut conn = mesh.write_connectivity();
    let outgoing = conn.at_vertex(start).outgoing_halfedges()?;
    let incoming = conn.at_vertex(start).incoming_halfedges()?;

    if incoming.len() != 1 {
        bail!("start should have exactly one incoming halfedge")
    }
    if outgoing.len() != 1 {
        bail!("start should have exactly one outgoing halfedge")
    }

    let e_inc = incoming[0];
    let e_out = outgoing[0];

    let end_v = conn.alloc_vertex(&mut mesh.write_positions(), end, None);

    let h_start_end = conn.alloc_halfedge(HalfEdge {
        vertex: Some(start),
        ..Default::default()
    });
    let h_end_start = conn.alloc_halfedge(HalfEdge {
        vertex: Some(end_v),
        ..Default::default()
    });

    conn[h_start_end].twin = Some(h_end_start);
    conn[h_start_end].next = Some(h_end_start);

    conn[h_end_start].twin = Some(h_start_end);
    conn[h_end_start].next = Some(e_out);

    conn[e_inc].next = Some(h_start_end);

    conn[end_v].halfedge = Some(h_end_start);

    Ok(end_v)
}

/// Adds an empty vertex to the mesh. Useful when the mesh is representing a
/// point cloud. Otherwise it's preferrable to use higher-level operators
pub fn add_vertex(this: &mut HalfEdgeMesh, pos: Vec3) -> Result<()> {
    this.write_connectivity()
        .alloc_vertex(&mut this.write_positions(), pos, None);
    Ok(())
}

/// Returns a point cloud mesh, selecting a set of vertices from the given mesh
pub fn point_cloud(mesh: &HalfEdgeMesh, sel: SelectionExpression) -> Result<HalfEdgeMesh> {
    let vertices = mesh.resolve_vertex_selection_full(&sel)?;
    let positions = mesh.read_positions();

    let new_mesh = HalfEdgeMesh::new();
    let mut new_conn = new_mesh.write_connectivity();
    let mut new_pos = new_mesh.write_positions();
    for v in vertices {
        new_conn.alloc_vertex(&mut new_pos, positions[v], None);
    }
    drop(new_conn);
    drop(new_pos);
    Ok(new_mesh)
}

pub fn vertex_attribute_transfer<V: ChannelValue>(
    src_mesh: &HalfEdgeMesh,
    dst_mesh: &mut HalfEdgeMesh,
    channel_name: &str,
) -> Result<()> {
    use rstar::{PointDistance, RTree, RTreeObject, AABB};

    // This is not that difficult to support, I just didn't have time to do it.
    // If done naively, this would lead to a double-borrow error on the channel.
    if channel_name == "position" {
        bail!("Attribute transfer using the 'position' channel is currently unsupported.")
    }

    // Retrieve the channel ids early so we can error if they don't exist.
    let src_channel_id = src_mesh
        .channels
        .channel_id::<VertexId, V>(channel_name)
        .ok_or_else(|| anyhow!("Source mesh has no channel called '{channel_name}'"))?;
    let dst_channel_id = dst_mesh
        .channels
        .ensure_channel::<VertexId, V>(channel_name);

    // Build a spatial index for the vertices in the source mesh. This takes
    // O(n) but in turn allows very efficient nearest-neighbor queries.
    pub struct VertexPos {
        vertex: VertexId,
        pos: Vec3,
    }

    impl RTreeObject for VertexPos {
        type Envelope = AABB<[f32; 3]>;
        fn envelope(&self) -> Self::Envelope {
            AABB::from_point(self.pos.to_array())
        }
    }

    impl PointDistance for VertexPos {
        fn distance_2(
            &self,
            point: &<Self::Envelope as rstar::Envelope>::Point,
        ) -> <<Self::Envelope as rstar::Envelope>::Point as rstar::Point>::Scalar {
            self.pos.distance_squared(Vec3::from_slice(point))
        }
    }

    let tree_index = RTree::bulk_load(
        src_mesh
            .read_connectivity()
            .iter_vertices_with_channel(&src_mesh.read_positions())
            .map(|(v_id, _, pos)| VertexPos { vertex: v_id, pos })
            .collect_vec(),
    );

    let src_channel = src_mesh.channels.read_channel(src_channel_id)?;
    let mut dst_channel = dst_mesh.channels.write_channel(dst_channel_id)?;
    for (dst_v, _, dst_pos) in dst_mesh
        .read_connectivity()
        .iter_vertices_with_channel(&dst_mesh.read_positions())
    {
        let nearest = tree_index
            .nearest_neighbor(&dst_pos.to_array())
            .ok_or_else(|| anyhow!("No nearest neighbor"))?;
        let src_value = src_channel[nearest.vertex];
        dst_channel[dst_v] = src_value;
    }

    Ok(())
}

pub fn set_material(
    mesh: &mut HalfEdgeMesh,
    selection: &SelectionExpression,
    material: f32,
) -> Result<()> {
    // TODO: Use default channels?
    let ch_id = mesh.channels.ensure_channel::<FaceId, f32>("material");
    let mut material_ch = mesh.channels.write_channel(ch_id)?;
    let ids = mesh.resolve_face_selection_full(selection)?;
    for id in ids {
        material_ch[id] = material;
    }
    Ok(())
}

/// TODO: Remove this once #[feature(map_first_last)] stabilizes
pub trait MapPolyfill<T> {
    fn pop_first2(&mut self) -> Option<T>;
}
impl<K, V> MapPolyfill<(K, V)> for BTreeMap<K, V>
where
    K: Clone + Ord,
{
    fn pop_first2(&mut self) -> Option<(K, V)> {
        let k = self.keys().next()?.clone();
        let v = self.remove(&k)?;
        Some((k, v))
    }
}
impl<T> MapPolyfill<T> for BTreeSet<T>
where
    T: Clone + Ord,
{
    fn pop_first2(&mut self) -> Option<T> {
        let k = self.iter().next()?.clone();
        self.remove(&k);
        Some(k)
    }
}

pub fn copy_to_points(points: &HalfEdgeMesh, cpy_mesh: &HalfEdgeMesh) -> Result<HalfEdgeMesh> {
    let conn = points.read_connectivity();
    let position_ch = points.read_positions();
    let size_ch = points
        .channels
        .read_channel_by_name::<VertexId, f32>("size");
    let normal_ch = points
        .channels
        .read_channel_by_name::<VertexId, Vec3>("normal");
    let tangent_ch = points
        .channels
        .read_channel_by_name::<VertexId, Vec3>("tangent");

    let mut result = HalfEdgeMesh::new();
    for (i, (v, _)) in conn.iter_vertices().enumerate() {
        let mut cpy_instance = cpy_mesh.clone();
        let instance_idx_ch_id = cpy_instance.channels.create_channel("instance_idx")?;

        // Mark all halfedges of this instance with its index
        let cpy_instance_conn = cpy_instance.read_connectivity();
        let mut instance_idx_ch = cpy_instance.channels.write_channel(instance_idx_ch_id)?;
        for (h, _) in cpy_instance_conn.iter_halfedges() {
            instance_idx_ch[h] = i as f32;
        }

        let scale = if let Ok(ref size) = size_ch {
            Vec3::splat(size[v])
        } else {
            Vec3::ONE
        };

        let rotate =
            if let (Ok(normal_ch), Ok(tangent_ch)) = (normal_ch.as_ref(), tangent_ch.as_ref()) {
                let normal = normal_ch[v];
                let tangent = tangent_ch[v];
                let cotangent = normal.cross(tangent);
                let (_, rotate, _) = glam::Affine3A::from_cols(
                    cotangent.into(),
                    normal.into(),
                    tangent.into(),
                    glam::Vec3A::ZERO,
                )
                .to_scale_rotation_translation();
                rotate.to_euler(glam::EulerRot::XYZ).into()
            } else {
                Vec3::ZERO
            };

        // Drop the channels so we can mutate the whole mesh
        drop(cpy_instance_conn);
        drop(instance_idx_ch);

        transform(&cpy_instance, position_ch[v], rotate, scale)?;
        result.merge_with(&cpy_instance);
    }

    Ok(result)
}

pub fn extrude_along_curve(
    backbone: &HalfEdgeMesh,
    cross_section: &HalfEdgeMesh,
    flip: usize,
) -> Result<HalfEdgeMesh> {
    let backbone_conn = backbone.read_connectivity();
    let backbone_pos = backbone.read_positions();
    let backbone_size = backbone
        .channels
        .read_channel_by_name::<VertexId, f32>("size");
    let backbone_nrm = backbone
        .channels
        .read_channel_by_name::<VertexId, Vec3>("normal");
    let backbone_tgt = backbone
        .channels
        .read_channel_by_name::<VertexId, Vec3>("tangent");

    // Sort the vertices of the cross-section
    let csect_pos = cross_section.read_positions();
    let csect_conn = cross_section.read_connectivity();
    let bag = cross_section.resolve_halfedge_selection_full(&SelectionExpression::All)?;
    let (csect_chain, is_closed) = sort_bag_of_edges(&csect_conn, &bag)?;

    let mut positions = vec![];

    for (v, _) in backbone_conn.iter_vertices() {
        let scale = if let Ok(ref size) = backbone_size {
            Vec3::splat(size[v])
        } else {
            Vec3::ONE
        };

        let rotate = if let (Ok(normal_ch), Ok(tangent_ch)) =
            (backbone_nrm.as_ref(), backbone_tgt.as_ref())
        {
            let normal = normal_ch[v];
            let tangent = tangent_ch[v];
            let cotangent = normal.cross(tangent);
            let (_, rotate, _) = glam::Affine3A::from_cols(
                cotangent.into(),
                normal.into(),
                tangent.into(),
                glam::Vec3A::ZERO,
            )
            .to_scale_rotation_translation();
            rotate.to_euler(glam::EulerRot::XYZ).into()
        } else {
            Vec3::ZERO
        };

        for vc in csect_chain.iter_cpy() {
            let pos = csect_pos[vc];
            let rot = Quat::from_euler(glam::EulerRot::XYZ, rotate.x, rotate.y, rotate.z);
            positions.push(rot * (pos * scale) + backbone_pos[v]);
        }
    }

    let mut polygons: Vec<[u32; 4]> = vec![];
    let num_segments = backbone_conn.num_vertices();
    let segment_length = csect_conn.num_vertices();

    for seg in 0..num_segments - 1 {
        let offset = seg * segment_length;
        for (i, j) in (0..segment_length as u32).branch(
            is_closed,
            |x| x.circular_tuple_windows(),
            |x| x.tuple_windows(),
        ) {
            let polygon = if flip % 2 == 0 {
                [i, j, j + segment_length as u32, i + segment_length as u32]
            } else {
                [j, i, i + segment_length as u32, j + segment_length as u32]
            }
            .map(|i| i + offset as u32);
            polygons.push(polygon);
        }
    }

    HalfEdgeMesh::build_from_polygons(&positions, &polygons)
}

pub enum ResampleCurveDensity {
    /// The curve will be sampled as uniform-length segments, taking the real
    /// (estimated) length of the curve into account.
    Uniform { segment_length: f32 },
    /// For each segment in the original curve, the resampled curve will have
    /// more segments the more curved the corresponding spline segment has.
    Curvature { multiplier: f32 },
}

pub fn resample_curve(
    mesh: &HalfEdgeMesh,
    density_mode: ResampleCurveDensity,
    tension: f32,
    alpha: f32,
) -> Result<HalfEdgeMesh> {
    /// Can be used to interpolate over a catmull rom segment in the polynomial
    /// form `p(t) = at³ + bt² + ct + d`
    pub struct CatmullRomSegment<const LUT_LEN: usize = 8> {
        a: Vec3,
        b: Vec3,
        c: Vec3,
        d: Vec3,
        // Lookup table for arc-length distances of evenly-spaced t values
        // ranging from 0 to 1.
        arc_length_lut: [f32; LUT_LEN],
        avg_curvature: f32,
    }

    impl<const LUT_LEN: usize> CatmullRomSegment<LUT_LEN> {
        pub fn position_at_t(&self, t: f32) -> Vec3 {
            self.a * (t * t * t) + self.b * (t * t) + self.c * t + self.d
        }

        pub fn tangent_at_t(&self, t: f32) -> Vec3 {
            self.a * (3.0 * t * t) + self.b * (2.0 * t) + self.c
        }

        pub fn acceleration_at_t(&self, t: f32) -> Vec3 {
            self.a * (6.0 * t) + self.b * 2.0
        }

        pub fn curvature_at_t(&self, t: f32) -> f32 {
            let d = self.tangent_at_t(t);
            let d_len = d.length();
            let d2 = self.acceleration_at_t(t);
            let result = (d.cross(d2)).length() / (d_len * d_len * d_len);
            if result.is_nan() {
                0.0
            } else {
                result
            }
        }

        pub fn arc_length(&self) -> f32 {
            self.arc_length_lut[LUT_LEN - 1]
        }

        pub fn t_for_arc_length(&self, length: f32) -> f32 {
            if length == 0.0 {
                0.0
            } else {
                let first_over = self
                    .arc_length_lut
                    .iter()
                    .position(|x| *x >= length)
                    .unwrap_or(1);
                assert!(first_over > 0, "Invariant");
                let last_under = first_over - 1;

                let l_first_over = self.arc_length_lut[first_over];
                let l_last_under = self.arc_length_lut[last_under];

                let t = (length - l_last_under) / (l_first_over - l_last_under);

                lerp(
                    last_under as f32 / (LUT_LEN - 1) as f32,
                    first_over as f32 / (LUT_LEN - 1) as f32,
                    t,
                )
            }
        }

        /// Computes the average curvature, by computing the curvature at
        /// uniform T intervals from 0 to LUT_LEN. The higher this value, the
        /// more "bent" this segment is.
        pub fn compute_average_curvature(&mut self) {
            self.avg_curvature = (0..LUT_LEN)
                .map(|i| {
                    let t = i as f32 / (LUT_LEN - 1) as f32;
                    self.curvature_at_t(t)
                })
                .sum::<f32>()
                / LUT_LEN as f32
        }

        fn average_curvature(&self) -> f32 {
            self.avg_curvature
        }

        /// Creates a new Catmull-Rom curve segment using control points p0..p4.
        /// The curve will go from p1 to p2. The p0 and p3 control points are
        /// chosen as the previous and next points in the curve respectively.
        pub fn new(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, tension: f32, alpha: f32) -> Self {
            // NOTE: This code was adapted from:
            // https://qroph.github.io/2018/07/30/smooth-paths-using-catmull-rom-splines.html
            let t0 = 0.0;
            let t1 = t0 + p0.distance(p1).powf(alpha);
            let t2 = t1 + p1.distance(p2).powf(alpha);
            let t3 = t2 + p2.distance(p3).powf(alpha);

            let m1 = (1.0 - tension)
                * (t2 - t1)
                * ((p1 - p0) / (t1 - t0) - (p2 - p0) / (t2 - t0) + (p2 - p1) / (t2 - t1));
            let m2 = (1.0 - tension)
                * (t2 - t1)
                * ((p2 - p1) / (t2 - t1) - (p3 - p1) / (t3 - t1) + (p3 - p2) / (t3 - t2));

            let a = 2.0 * (p1 - p2) + m1 + m2;
            let b = -3.0 * (p1 - p2) - m1 - m1 - m2;
            let c = m1;
            let d = p1;

            let f = |t| a * (t * t * t) + b * (t * t) + c * t + d;

            // The formula for the arc length is not easy to compute, so we
            // approximate it by storing a lookup table
            let mut arc_length_lut = [0.0; LUT_LEN]; // NOTE: First value is correct at 0.0
            for i in 1..LUT_LEN {
                let t_i_prev = (i - 1) as f32 / (LUT_LEN - 1) as f32;
                let t_i = i as f32 / (LUT_LEN - 1) as f32;
                arc_length_lut[i] = arc_length_lut[i - 1] + f(t_i_prev).distance(f(t_i));
            }

            let mut segment = CatmullRomSegment {
                a,
                b,
                c,
                d,
                arc_length_lut,
                avg_curvature: 0.0, // Filled in later
            };

            segment.compute_average_curvature();
            segment
        }
    }

    match density_mode {
        ResampleCurveDensity::Uniform { segment_length } => {
            if segment_length <= 0.0 {
                bail!("Resolution must be greater than zero");
            }
        }
        ResampleCurveDensity::Curvature { multiplier } => {
            if multiplier <= 0.0 {
                bail!("Curvature multiplier must be greater than zero");
            }
        }
    }

    // Make sure the input mesh is a curve and find its endpoints.
    let edges = mesh.resolve_halfedge_selection_full(&SelectionExpression::All)?;
    let (curve, is_closed) = sort_bag_of_edges(&mesh.read_connectivity(), &edges)?;
    let np = curve.len();

    if curve.len() < 2 {
        bail!("A curve can only be resampled if it has 2 or more points")
    }

    if is_closed {
        bail!("TODO: Resampling closed curves is currently unimplemented.")
    }

    let positions = mesh.write_positions();
    let p_first = positions[curve[0]] + (positions[curve[1]] - positions[curve[0]]);
    let p_last = positions[curve[np - 1]] + (positions[curve[np - 1]] - positions[curve[np - 2]]);

    let control_points = std::iter::once(p_first)
        .chain(curve.iter().map(|x| positions[*x]))
        .chain(std::iter::once(p_last));

    let mut points = vec![];
    let mut tangents = vec![];
    let mut curvatures = vec![];
    let mut accelerations = vec![];
    let mut offset = 0.0;
    for (p0, p1, p2, p3) in control_points.tuple_windows() {
        let segment = CatmullRomSegment::<8>::new(p0, p1, p2, p3, tension, alpha);

        let resolution = match density_mode {
            ResampleCurveDensity::Uniform { segment_length } => segment_length,
            ResampleCurveDensity::Curvature { multiplier } => {
                let avg_curvature = segment.average_curvature().max(1.0); // Prevent division by 0
                (1.0 / avg_curvature) * multiplier
            }
        };

        // Could be that previous iteration produced an offset that is too long
        // for this segment. In that case we simply don't use any points from
        // this segment.
        if offset > segment.arc_length() {
            offset = (offset + segment.arc_length()) % resolution;
            continue;
        }

        let total_dist = segment.arc_length() - offset;
        let nsegments = (total_dist / resolution).floor();

        // ..= because there's n+1 points inside n segments.
        for i in 0..=nsegments as u32 {
            let t = segment.t_for_arc_length(resolution * i as f32 + offset);
            points.push(segment.position_at_t(t));
            tangents.push(segment.tangent_at_t(t));
            curvatures.push(segment.curvature_at_t(t));
            accelerations.push(segment.acceleration_at_t(t));
        }

        offset = resolution - (total_dist - (nsegments * resolution));
    }

    if points.len() < 2 {
        bail!("Resolution is too low, curve has less than two points.");
    }

    // Manually drop to avoid double borrow inside add_edge
    drop(positions);

    let mut result_mesh = HalfEdgeMesh::new();
    let tangent_ch_id = result_mesh.channels.ensure_channel("tangent");
    let normal_ch_id = result_mesh.channels.ensure_channel("normal");
    let curvature_ch_id = result_mesh.channels.ensure_channel("curvature");
    let acc_ch_id = result_mesh.channels.ensure_channel("acceleration");
    let mut tangent_ch = result_mesh.channels.write_channel(tangent_ch_id).unwrap();
    let mut normal_ch = result_mesh.channels.write_channel(normal_ch_id).unwrap();
    let mut curvature_ch = result_mesh.channels.write_channel(curvature_ch_id).unwrap();
    let mut acc_ch = result_mesh.channels.write_channel(acc_ch_id).unwrap();

    // Add the first edge
    let (h_src, h_dst) = add_edge(&result_mesh, points[0], points[1])?;
    {
        // And the tangents and normals for the first edge
        let v0 = mesh.read_connectivity().at_halfedge(h_src).vertex().end();
        let v1 = mesh.read_connectivity().at_halfedge(h_dst).vertex().end();
        tangent_ch[v0] = tangents[0];
        tangent_ch[v1] = tangents[1];

        normal_ch[v0] = tangents[0].cross(Vec3::Y);
        normal_ch[v1] = tangents[1].cross(Vec3::Y);

        curvature_ch[v0] = curvatures[0];
        curvature_ch[v1] = curvatures[1];

        acc_ch[v0] = accelerations[0];
        acc_ch[v1] = accelerations[1];
    }

    // Add the remaining edges
    let mut v = mesh.read_connectivity().at_halfedge(h_dst).vertex().end();
    for (((dst, dst_tg), dst_crv), dst_jrk) in points
        .iter_cpy()
        .zip(tangents.iter_cpy())
        .zip(curvatures.iter_cpy())
        .zip(accelerations.iter_cpy())
        .dropping(2)
    {
        v = add_edge_chain(&result_mesh, v, dst)?;
        tangent_ch[v] = dst_tg;
        normal_ch[v] = dst_tg.cross(Vec3::Y);
        curvature_ch[v] = dst_crv;
        acc_ch[v] = dst_jrk;
    }

    drop(tangent_ch);
    drop(normal_ch);
    drop(curvature_ch);
    drop(acc_ch);
    Ok(result_mesh)
}

pub fn edit_geometry(
    mesh: &mut HalfEdgeMesh,
    geometry_type: ChannelKeyType,
    selection: SelectionExpression,
    translate: Vec3,
    rotate: Vec3,
    scale: Vec3,
) -> Result<()> {
    let conn = mesh.read_connectivity();
    let vertices = match geometry_type {
        ChannelKeyType::VertexId => mesh.resolve_vertex_selection_full(&selection)?,
        ChannelKeyType::FaceId => mesh
            .resolve_face_selection_full(&selection)?
            .iter()
            .flat_map(|f| conn.at_face(*f).vertices())
            .flatten()
            .unique()
            .collect_vec(),
        ChannelKeyType::HalfEdgeId => mesh
            .resolve_halfedge_selection_full(&selection)?
            .iter()
            .flat_map(|h| conn.at_halfedge(*h).src_dst_pair())
            .flat_map(|(a, b)| [a, b])
            .unique()
            .collect_vec(),
    };

    let mut pos = mesh.write_positions();
    let centroid = vertices
        .iter()
        .map(|v| pos[*v])
        .fold(Vec3::ZERO, |v, v2| v + v2)
        / vertices.len() as f32;

    let transform_matrix = Mat4::from_translation(centroid)
        * Mat4::from_scale_rotation_translation(
            scale,
            Quat::from_euler(EulerRot::XYZ, rotate.x, rotate.y, rotate.z),
            translate,
        )
        * Mat4::from_translation(-centroid);

    for v in vertices {
        pos[v] = transform_matrix.transform_point3(pos[v]);
    }

    Ok(())
}

#[blackjack_macros::blackjack_lua_module]
pub mod lua_fns {

    use crate::lua_engine::lua_stdlib::LVec3;
    use halfedge::compact_mesh::CompactMesh;

    use super::*;

    /// Replaces each vertex in the `vertices` selection with a face, and moves
    /// that face along the incident edges by a given `amount` distance.
    #[lua(under = "Ops")]
    pub fn chamfer(
        vertices: SelectionExpression,
        amount: f32,
        mesh: &mut HalfEdgeMesh,
    ) -> Result<()> {
        mesh.write_connectivity().clear_debug();
        let verts = mesh.resolve_vertex_selection_full(&vertices)?;
        for v in verts {
            crate::mesh::halfedge::edit_ops::chamfer_vertex(
                &mut mesh.write_connectivity(),
                &mut mesh.write_positions(),
                v,
                amount,
            )?;
        }
        Ok(())
    }

    /// Bevels the given `edges`, replacing each edge with a face and indenting
    /// it by a given `amount` distance.
    #[lua(under = "Ops")]
    pub fn bevel(edges: SelectionExpression, amount: f32, mesh: &HalfEdgeMesh) -> Result<()> {
        let edges = mesh.resolve_halfedge_selection_full(&edges)?;
        crate::mesh::halfedge::edit_ops::bevel_edges(
            &mut mesh.write_connectivity(),
            &mut mesh.write_positions(),
            &edges,
            amount,
        )
    }

    /// Extrudes the given `faces` by a given `amount` distance.
    #[lua(under = "Ops")]
    pub fn extrude(faces: SelectionExpression, amount: f32, mesh: &HalfEdgeMesh) -> Result<()> {
        let faces = mesh.resolve_face_selection_full(&faces)?;
        crate::mesh::halfedge::edit_ops::extrude_faces(
            &mut mesh.write_connectivity(),
            &mut mesh.write_positions(),
            &faces,
            amount,
        )?;
        Ok(())
    }

    /// Extrudes the given `faces` by a given `amount` distance and flips the normal of the
    /// original faces in order to cap the extrusion into a solid.
    #[lua(under = "Ops")]
    pub fn extrude_with_caps(
        faces: SelectionExpression,
        amount: f32,
        mesh: &mut HalfEdgeMesh,
    ) -> Result<()> {
        let faces = mesh.resolve_face_selection_full(&faces)?;
        let face_set: HashSet<FaceId> = faces.iter().cloned().collect();

        let mut to_merge = vec![];
        {
            // For each face, a new face is created with the opposite winding order so that
            // its normal faces opposite to the original face.
            let mesh_connectivity = &mesh.write_connectivity();
            let mesh_positions = &mesh.write_positions();
            for f in face_set {
                let mut reversed_points = mesh_connectivity.at_face(f).vertices()?;
                reversed_points.reverse();
                let vec3s = reversed_points
                    .iter()
                    .enumerate()
                    .map(|(_, v_id)| mesh_positions[*v_id])
                    .collect_vec();
                let indices = reversed_points
                    .iter()
                    .enumerate()
                    .map(|(i, _)| i as u32)
                    .collect_vec();
                let half_edge_mesh = HalfEdgeMesh::build_from_polygons(&vec3s, &[&indices])?;
                to_merge.push(half_edge_mesh);
            }
        }

        for half_edge_mesh in to_merge {
            mesh.merge_with(&half_edge_mesh);
        }

        crate::mesh::halfedge::edit_ops::extrude_faces(
            &mut mesh.write_connectivity(),
            &mut mesh.write_positions(),
            &faces,
            amount,
        )?;
        Ok(())
    }

    /// Modifies the given mesh `a` by merging `b` into it. The `b` mesh remains
    /// unmodified.
    #[lua(under = "Ops")]
    pub fn merge(a: &mut HalfEdgeMesh, b: &HalfEdgeMesh) -> Result<()> {
        a.merge_with(b);
        Ok(())
    }

    /// Subdivides the given mesh, applying as many `iterations` as given. If
    /// `catmull_clark` is true, will use catmull clark subdivision, else linear
    /// (i.e. vertex positions remain unchanged).
    #[lua(under = "Ops")]
    pub fn subdivide(
        mesh: &HalfEdgeMesh,
        iterations: usize,
        catmull_clark: bool,
    ) -> Result<HalfEdgeMesh> {
        let new_mesh = CompactMesh::<false>::from_halfedge(mesh)?;
        Ok(new_mesh
            .subdivide_multi(iterations, catmull_clark)
            .to_halfedge())
    }

    /// Computes the smooth normals channel for the given `mesh` and sets the
    /// mesh export settings to use smooth normals.
    #[lua(under = "Ops")]
    pub fn set_smooth_normals(mesh: &mut HalfEdgeMesh) -> Result<()> {
        super::set_smooth_normals(mesh)?;
        Ok(())
    }

    /// Computes the flat normals channel for the given `mesh` and sets the
    /// mesh export settings to use flat normals.
    #[lua(under = "Ops")]
    pub fn set_flat_normals(mesh: &mut HalfEdgeMesh) -> Result<()> {
        super::set_flat_normals(mesh)?;
        Ok(())
    }

    /// Given a mesh representing a polyline, resamples it using Catmull-Rom
    /// interpolation to create a smooth path that passes through all the points
    /// of the original curve.
    ///
    /// TODO: Obsolete docs!!!!!
    /// The `resolution` will be used to determine the distance between each
    /// pair of points in the final curve. Depending on the input curve and this
    /// value, the actual waypoints may not be part of the final resampled
    /// curve, but given a small enough segment length, the final curve will be
    /// a good approximation of the input waypoints.
    ///
    /// The `alpha` value can be set to 0 for a uniform, 0.5 for a centripetal
    /// and 1.0 for a chordal Catmull-Rom spline. If in doubt, pick 0.5 for good
    /// results.
    ///
    /// Increasing the `tension` from 0 to 1 value will make the curves more
    /// pronounced, as if it were increasing the tension of a rope that goes
    /// through all the points. A good value for tension is 0.5
    #[lua(under = "Ops")]
    pub fn resample_curve(
        mesh: &HalfEdgeMesh,
        density_mode: String,
        density: f32,
        tension: f32,
        alpha: f32,
    ) -> Result<HalfEdgeMesh> {
        let density_mode = if density_mode == "Uniform" {
            ResampleCurveDensity::Uniform {
                segment_length: density,
            }
        } else if density_mode == "Curvature" {
            ResampleCurveDensity::Curvature {
                multiplier: density,
            }
        } else {
            bail!("Invalid density mode: {density_mode}")
        };

        super::resample_curve(mesh, density_mode, tension, alpha)
    }

    /// Given two edge selections, bridges the two edge selections with quads
    /// spanning every pair of consecutive edges.
    ///
    /// The `flip` parameter can be used to select a permutation for the winding
    /// order of each of the input loops.
    #[lua(under = "Ops")]
    pub fn bridge_chains(
        mesh: &mut HalfEdgeMesh,
        loop_1: SelectionExpression,
        loop_2: SelectionExpression,
        flip: usize,
    ) -> Result<()> {
        let bag_1 = mesh.resolve_halfedge_selection_full(&loop_1)?;
        let bag_2 = mesh.resolve_halfedge_selection_full(&loop_2)?;
        super::bridge_chains_ui(mesh, &bag_1, &bag_2, flip)
    }

    /// Given four vertices `a`, `b`, `c` and `d`, creates a quad face between
    /// these vertices. This operation will fail if the operation would lead to
    /// a non-manifold mesh, or if any of the a->b b->c c->d or d->a halfedges
    /// is already part of a face.
    #[lua(under = "Ops")]
    pub fn make_quad(
        mesh: &mut HalfEdgeMesh,
        a: SelectionExpression,
        b: SelectionExpression,
        c: SelectionExpression,
        d: SelectionExpression,
    ) -> Result<()> {
        macro_rules! get_selection {
            ($sel:expr) => {
                mesh.resolve_vertex_selection_full(&$sel)?
                    .get(0)
                    .copied()
                    .ok_or_else(|| anyhow::anyhow!("Empty selection"))?
            };
        }

        let a = get_selection!(a);
        let b = get_selection!(b);
        let c = get_selection!(c);
        let d = get_selection!(d);

        super::make_quad(&mut mesh.write_connectivity(), &[a, b, c, d])
    }

    /// Applies a transformation to the `position` channel of this mesh, by
    /// translating, rotating and scaling the mesh with given parameters.
    #[lua(under = "Ops")]
    pub fn transform(
        mesh: &mut HalfEdgeMesh,
        translate: LVec3,
        rotate: LVec3,
        scale: LVec3,
    ) -> Result<()> {
        super::transform(mesh, translate.0, rotate.0, scale.0)
    }

    /// Creates a group named `group_name` in `mesh` for the given mesh element
    /// `key_type`. This will put all the elements in `selection` inside this
    /// group.
    ///
    /// A group is simply a boolean channel storing `true` values for every
    /// element in the group, so this method is only a convenient wrapper over
    /// the more general `edit_channels`.
    #[lua(under = "Ops")]
    pub fn make_group(
        mesh: &mut HalfEdgeMesh,
        key_type: ChannelKeyType,
        selection: SelectionExpression,
        group_name: String,
    ) -> Result<()> {
        super::make_group(mesh, key_type, &selection, &group_name)
    }

    /// Sets the `material` channel for all faces in `selection` to use the
    /// given `material_index`.
    ///
    /// Material indices are currently not interpreted by blackjack, but game
    /// engine integrations may use this channel in different ways.
    #[lua(under = "Ops")]
    pub fn set_material(
        mesh: &mut HalfEdgeMesh,
        selection: SelectionExpression,
        material_index: f32,
    ) -> Result<()> {
        super::set_material(mesh, &selection, material_index)
    }

    /// Given a source mesh (`src_mesh`) and a destination mesh (`dst_mesh`),
    /// transfers the vertex channel with given `value_type` and `channel_name`
    /// from source to mesh.
    ///
    /// Transfer for a vertex of the source mesh works by copying the value of
    /// the nearest vertex of the destination mesh.
    #[lua(under = "Ops")]
    pub fn vertex_attribute_transfer(
        src_mesh: &HalfEdgeMesh,
        dst_mesh: &mut HalfEdgeMesh,
        value_type: ChannelValueType,
        channel_name: String,
    ) -> Result<()> {
        match value_type {
            ChannelValueType::Vec3 => {
                super::vertex_attribute_transfer::<glam::Vec3>(src_mesh, dst_mesh, &channel_name)
            }
            ChannelValueType::f32 => {
                super::vertex_attribute_transfer::<f32>(src_mesh, dst_mesh, &channel_name)
            }
            ChannelValueType::bool => {
                super::vertex_attribute_transfer::<bool>(src_mesh, dst_mesh, &channel_name)
            }
        }
    }

    /// Generates an UV channel (HalfEdgeId -> Vec3) for the mesh where ever
    /// polygon is mapped to the full UV range. Triangles will take half the UV
    /// space, quads will take the full space, and n-gons will take as much
    /// space as possible, being centered in the middle.
    #[lua(under = "Ops")]
    pub fn set_full_range_uvs(mesh: &mut HalfEdgeMesh) -> Result<()> {
        super::set_full_range_uvs(mesh)
    }

    /// Given a `points` mesh, taken as a point cloud and another `mesh`, returs
    /// a new mesh where `mesh` is instanced at every point of the point cloud.
    ///
    /// The following additional channels influence the behavior of this
    /// operation:
    ///
    /// - The `normal` and `tangent` vertex channels, if present, will be used
    /// to set the orientation of the cross-section at each point.
    /// - The `size` vertex channel will be used to scale the cross section at
    /// each point.
    #[lua(under = "Ops")]
    pub fn copy_to_points(points: &HalfEdgeMesh, mesh: &HalfEdgeMesh) -> Result<HalfEdgeMesh> {
        super::copy_to_points(points, mesh)
    }

    /// Given a `backbone` mesh and a cross-section mesh, both polylines,
    /// returns a new mesh which extrudes the cross-section across the backbone.
    ///
    /// The following additional channels influence the behavior of this
    /// operation:
    ///
    /// - The `normal` and `tangent` vertex channels, if present, will be used
    /// to set the orientation of the cross-section at each point.
    /// - The `size` vertex channel will be used to scale the cross section at
    /// each point.
    #[lua(under = "Ops")]
    pub fn extrude_along_curve(
        backbone: &HalfEdgeMesh,
        cross_section: &HalfEdgeMesh,
        flip: usize,
    ) -> Result<HalfEdgeMesh> {
        super::extrude_along_curve(backbone, cross_section, flip)
    }

    /// Applies a transformation to the given selection of mesh elements
    /// (vertex, face, halfedge). The transformation is applied relative to the
    /// elements centroid.
    #[lua(under = "Ops")]
    pub fn edit_geometry(
        mesh: &mut HalfEdgeMesh,
        geometry_type: ChannelKeyType,
        selection: SelectionExpression,
        translate: LVec3,
        rotate: LVec3,
        scale: LVec3,
    ) -> Result<()> {
        super::edit_geometry(
            mesh,
            geometry_type,
            selection,
            translate.0,
            rotate.0,
            scale.0,
        )
    }

    /// Collapses an `edge`, fusing the source and destination vertices in to one.
    /// If this operation is applied to a triangle, the face will be removed and
    /// become a single edge.o
    ///
    /// The position of the new collapsed vertex will be interpolated between
    /// the vertices of the original halfedge with a given `interpolation`
    /// factor.
    #[lua(under = "Ops")]
    pub fn collapse_edge(
        mesh: &mut HalfEdgeMesh,
        edges: SelectionExpression,
        interpolation: f32,
    ) -> Result<()> {
        let edges = mesh.resolve_halfedge_selection_full(&edges)?;
        for edge in edges {
            let mut positions = mesh.write_positions();
            let (src, dst) = mesh.read_connectivity().at_halfedge(edge).src_dst_pair()?;
            let vpos = lerp(positions[src], positions[dst], interpolation);

            let v = super::collapse_edge(&mut mesh.write_connectivity(), edge)?;

            positions[v] = vpos;
        }

        Ok(())
    }

    #[lua(under = "Ops")]
    pub fn divide_edges(
        mesh: &mut HalfEdgeMesh,
        edges: SelectionExpression,
        interpolation: f32,
        divisions: usize,
    ) -> Result<()> {
        let edges = mesh.resolve_halfedge_selection_full(&edges)?;
        for edge in edges {
            if divisions == 1 {
                super::divide_edge(
                    &mut mesh.write_connectivity(),
                    &mut mesh.write_positions(),
                    edge,
                    interpolation,
                )?;
            } else {
                let edge_vector = {
                    let (src, dst) = mesh.read_connectivity().at_halfedge(edge).src_dst_pair()?;
                    let positions = mesh.read_positions();
                    positions[dst] - positions[src]
                };
                let vs = (0..divisions)
                    .map(|_| {
                        super::divide_edge(
                            &mut mesh.write_connectivity(),
                            &mut mesh.write_positions(),
                            edge,
                            0.0,
                        )
                    })
                    .collect::<Result<Vec<_>>>()?;

                let mut positions = mesh.write_positions();
                for (i, v) in vs.iter_cpy().enumerate() {
                    positions[v] += edge_vector * (i as f32 / divisions as f32);
                }
            }
        }

        Ok(())
    }

    #[lua(under = "Ops")]
    pub fn cut_face(
        mesh: &mut HalfEdgeMesh,
        a: SelectionExpression,
        b: SelectionExpression,
    ) -> Result<HalfEdgeId> {
        macro_rules! get_selection {
            ($sel:expr) => {
                mesh.resolve_vertex_selection_full(&$sel)?
                    .get(0)
                    .copied()
                    .ok_or_else(|| anyhow::anyhow!("Empty selection"))?
            };
        }

        let a = get_selection!(a);
        let b = get_selection!(b);

        let h = super::cut_face(&mut mesh.write_connectivity(), a, b)?;

        Ok(h)
    }
}
