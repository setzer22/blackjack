// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

/*
/// This map is used in many operations when halfedges are still being built.
/// Sometimes we need to keep this information to locate twins, and using
/// `halfedge_to` won't work because we can't cycle the edges around a vertex
/// fan until twins are assigned.
type PairToHalfEdge = std::collections::HashMap<(VertexId, VertexId), HalfEdgeId>;

/// Given a list of vertices, forms a face with all of them. To call this
/// function, the vertices should be in the right winding order and must all be
/// part of the same boundary. If not, it will panic or may produce wrong results.
fn add_face(
    mesh: &mut HalfEdgeMesh,
    vertices: &[VertexId],
    pair_to_halfedge: &mut PairToHalfEdge,
) -> FaceId {
    let mut halfedges = SVec::new();

    let f = mesh.alloc_face(None);

    for (&v, &v2) in vertices.iter().circular_tuple_windows() {
        // Some vertices may already be connected by an edge. We should avoid
        // creating halfedges for those.
        let h = if let Some(&h) = pair_to_halfedge.get(&(v, v2)) {
            // The halfedge may exist, but the face could've changed.
            mesh[h].face = Some(f);
            h
        } else {
            mesh.alloc_halfedge(HalfEdge {
                vertex: Some(v),
                face: Some(f),
                twin: None, // TBD
                next: None, // TBD
            })
        };
        pair_to_halfedge.insert((v, v2), h);
        halfedges.push(h);
        mesh[v].halfedge = Some(h);
    }

    for (&ha, &hb) in halfedges.iter().circular_tuple_windows() {
        mesh[ha].next = Some(hb);
    }

    mesh[f].halfedge = Some(halfedges[0]);

    // For each pair of vertices a,b and the halfedge that goes from a to b,
    // a_h_b, we attempt to find its twin, that is, the edge in the mesh that
    // goes from b to a. If found, we link it to the one we created.
    //
    // NOTE: Both the halfedge and its twin may already exist and be linked. In
    // that case, they are simply reassigned. If the twin does not exist,
    // nothing happens, it may be linked later as part of anoter add_face
    // operation.
    for ((&a, &b), h_a_b) in vertices.iter().circular_tuple_windows().zip(halfedges) {
        if let Some(&h_b_a) = pair_to_halfedge.get(&(b, a)) {
            mesh[h_b_a].twin = Some(h_a_b);
            mesh[h_a_b].twin = Some(h_b_a);
        }
    }

    f
}

pub fn extrude_face_connectivity(
    mesh: &mut HalfEdgeMesh,
    face_id: FaceId,
    position_delta: Vec3,
) -> (SVec<FaceId>, FaceId) {
    let vertices = mesh.at_face(face_id).vertices().unwrap();
    let halfedges = mesh.at_face(face_id).halfedges().unwrap();

    let mut new_vertices = SVec::new();
    for &v in vertices.iter() {
        let pos = mesh.vertex_position(v);
        new_vertices.push(mesh.alloc_vertex(pos + position_delta, None));
    }

    // NOTE: It's important to initialize this structure, or some halfedges
    // would get duplicated.
    let mut pair_to_halfedge: PairToHalfEdge = vertices
        .iter()
        .cloned()
        .circular_tuple_windows()
        .zip(halfedges.iter().cloned())
        .collect();

    let mut side_faces = SVec::new();

    // v1->v2 is the direction of the existing halfedges. We need to follow that
    // same direction to preserve mesh orientation.
    for ((&v1, &v1_new), (&v2, &v2_new)) in vertices
        .iter()
        .zip(new_vertices.iter())
        .circular_tuple_windows()
    {
        side_faces.push(add_face(
            mesh,
            &[v1, v2, v2_new, v1_new],
            &mut pair_to_halfedge,
        ));
    }

    // TODO: Maybe reuse the old face?
    let front_face = add_face(mesh, new_vertices.as_slice(), &mut pair_to_halfedge);

    mesh.faces.remove(face_id.0);

    #[cfg(debug_assertions)]
    for halfedge in halfedges {
        debug_assert!(
            mesh[halfedge].face.unwrap() != face_id,
            "None of the original halfedges should point to the old face"
        );
    }

    for vertex in mesh.at_face(front_face).vertices().unwrap().iter() {
        mesh.add_debug_vertex(*vertex, DebugMark::new("ex", egui::Color32::RED));
    }

    (side_faces, front_face)
}

pub const ORANGE: egui::Color32 = egui::Color32::from_rgb(200, 200, 0);

#[allow(non_snake_case)]
pub fn split_vertex(
    mesh: &mut HalfEdgeMesh,
    v: VertexId,
    v_l: VertexId,
    v_r: VertexId,
    delta: Vec3,
    dbg: bool,
) -> Result<VertexId> {
    let v_pos = mesh.vertex_position(v);

    // Find h_L and h_R
    let h_r_v = mesh.at_vertex(v_r).halfedge_to(v).try_end()?;
    let h_v_r = mesh.at_halfedge(h_r_v).twin().end();
    let h_v_l = mesh.at_vertex(v).halfedge_to(v_l).try_end()?;
    let h_l_v = mesh.at_halfedge(h_v_l).twin().end();

    if dbg {
        mesh.add_debug_halfedge(h_r_v, DebugMark::new("h_R", ORANGE));
        mesh.add_debug_halfedge(h_v_l, DebugMark::new("h_L", ORANGE));
    }

    // Get all the halfedges edges connected to v starting at v_r and ending at
    // v_l in clockwise order
    let (incoming_hs, outgoing_hs) = {
        let incoming = mesh.at_vertex(v).incoming_halfedges()?;
        let outgoing = mesh.at_vertex(v).outgoing_halfedges()?;

        let h_incoming_start = incoming.iter().position(|x| *x == h_r_v).unwrap();
        let h_incoming_end = incoming.iter().position(|x| *x == h_l_v).unwrap();
        let h_incoming_end = if h_incoming_end < h_incoming_start {
            h_incoming_end + incoming.len()
        } else {
            h_incoming_end
        };

        let incoming_hs: SVec<HalfEdgeId> = (h_incoming_start + 1..h_incoming_end)
            .map(|x| x % incoming.len())
            .map(|idx| incoming[idx])
            .collect();

        if dbg {
            for &h in &incoming_hs {
                mesh.add_debug_halfedge(h, DebugMark::new("", egui::Color32::BLUE));
            }
        }

        let h_outgoing_start = outgoing.iter().position(|x| *x == h_v_r).unwrap();
        let h_outgoing_end = outgoing.iter().position(|x| *x == h_v_l).unwrap();
        let h_outgoing_end = if h_outgoing_end < h_outgoing_start {
            h_outgoing_end + outgoing.len()
        } else {
            h_outgoing_end
        };
        let outgoing_hs: SVec<HalfEdgeId> = (h_outgoing_start + 1..h_outgoing_end)
            .map(|x| x % outgoing.len())
            .map(|idx| outgoing[idx])
            .collect();

        if dbg {
            for &h in &outgoing_hs {
                mesh.add_debug_halfedge(h, DebugMark::new("", egui::Color32::RED));
            }
        }
        (incoming_hs, outgoing_hs)
    };

    if dbg {
        mesh.add_debug_vertex(v, DebugMark::new("v", egui::Color32::RED));
        mesh.add_debug_vertex(v_l, DebugMark::new("v_L", egui::Color32::RED));
        mesh.add_debug_vertex(v_r, DebugMark::new("v_R", egui::Color32::RED));
    }

    // Get the face
    let f_l_old = if !mesh.at_halfedge(h_v_l).is_boundary()? {
        Some(mesh.at_halfedge(h_v_l).face().end())
    } else {
        None
    };
    let f_r_old = if !mesh.at_halfedge(h_r_v).is_boundary()? {
        Some(mesh.at_halfedge(h_r_v).face().end())
    } else {
        None
    };

    // These halfedges will need to get re-routed
    let prev_h_r_v = mesh.at_halfedge(h_r_v).previous().end();
    let next_h_v_l = mesh.at_halfedge(h_v_l).next().end();

    if dbg {
        mesh.add_debug_halfedge(prev_h_r_v, DebugMark::green("prev_h_r_v"));
        mesh.add_debug_halfedge(next_h_v_l, DebugMark::green("next_h_v_l"));
    }

    // Allocate *all* the new structures
    let w = mesh.alloc_vertex(v_pos + delta, None);
    let h_v_w = mesh.alloc_halfedge(HalfEdge::default());
    let h_w_v = mesh.alloc_halfedge(HalfEdge::default());
    let h_l_w = mesh.alloc_halfedge(HalfEdge::default());
    let h_w_l = mesh.alloc_halfedge(HalfEdge::default());
    let h_r_w = mesh.alloc_halfedge(HalfEdge::default());
    let h_w_r = mesh.alloc_halfedge(HalfEdge::default());
    let f_l = mesh.alloc_face(None);
    let f_r = mesh.alloc_face(None);

    // --- Create the new connectivity data ---

    // Left face
    mesh[h_w_v].next = Some(h_v_l);
    mesh[h_v_l].next = Some(h_l_w);
    mesh[h_l_w].next = Some(h_w_v);
    mesh[h_w_v].face = Some(f_l);
    mesh[h_v_l].face = Some(f_l);
    mesh[h_l_w].face = Some(f_l);

    // Right face
    mesh[h_v_w].next = Some(h_w_r);
    mesh[h_w_r].next = Some(h_r_v);
    mesh[h_r_v].next = Some(h_v_w);
    mesh[h_v_w].face = Some(f_r);
    mesh[h_w_r].face = Some(f_r);
    mesh[h_r_v].face = Some(f_r);

    // Vertices
    mesh[h_v_w].vertex = Some(v);
    mesh[h_w_v].vertex = Some(w);
    mesh[h_l_w].vertex = Some(v_l);
    mesh[h_w_l].vertex = Some(w);
    mesh[h_r_w].vertex = Some(v_r);
    mesh[h_w_r].vertex = Some(w);

    // Face / vertex links
    mesh[f_l].halfedge = Some(h_l_w);
    mesh[f_r].halfedge = Some(h_w_r);
    mesh[w].halfedge = Some(h_w_v);

    // Twins
    mesh[h_v_w].twin = Some(h_w_v);
    mesh[h_w_v].twin = Some(h_v_w);

    mesh[h_l_w].twin = Some(h_w_l);
    mesh[h_w_l].twin = Some(h_l_w);

    mesh[h_r_w].twin = Some(h_w_r);
    mesh[h_w_r].twin = Some(h_r_w);

    // --- Readjust old connectivity data ---

    // It is likely that the halfedges for v, or the L and R faces are no longer
    // valid. In order to avoid a linear scan check, we just reassign those to
    // values that we already know valid
    mesh[h_w_l].face = f_l_old; // Could be none for boundary
    if let Some(f_l_old) = f_l_old {
        mesh[f_l_old].halfedge = Some(h_w_l);
    }
    mesh[h_r_w].face = f_r_old; // Could be none for boundary
    if let Some(f_r_old) = f_r_old {
        mesh[f_r_old].halfedge = Some(h_r_w);
    }
    mesh[v].halfedge = Some(h_v_w);

    // Adjust next pointers
    mesh[prev_h_r_v].next = Some(h_r_w);

    mesh[h_w_l].next = Some(next_h_v_l);

    mesh[h_r_w].next = Some(*outgoing_hs.get(0).unwrap_or(&h_w_l));
    if !incoming_hs.is_empty() {
        mesh[incoming_hs[incoming_hs.len() - 1]].next = Some(h_w_l);
    }

    // Adjust outgoing halfedge origins
    for out_h in outgoing_hs {
        mesh[out_h].vertex = Some(w);
    }

    Ok(w)
}

pub fn split_edge(
    mesh: &mut HalfEdgeMesh,
    h: HalfEdgeId,
    delta: Vec3,
    dbg: bool,
) -> Result<HalfEdgeId> {
    let (v, w) = mesh.at_halfedge(h).src_dst_pair()?;

    // NOTE: Next edge in edge loop is computed as next-twin-next
    #[rustfmt::skip]
    let (v_prev, w_next) = {
        let v_prev = mesh.at_vertex(v).halfedge_to(w).previous().twin().previous().vertex().try_end()?;
        let w_next = mesh.at_vertex(w).halfedge_to(v).previous().twin().previous().vertex().try_end()?;
        (v_prev, w_next)
    };

    if dbg {
        mesh.add_debug_vertex(v_prev, DebugMark::new("v_prv", egui::Color32::BLUE));
        mesh.add_debug_vertex(v, DebugMark::new("v", egui::Color32::BLUE));
        mesh.add_debug_vertex(w, DebugMark::new("w", egui::Color32::BLUE));
        mesh.add_debug_vertex(w_next, DebugMark::new("w_next", egui::Color32::BLUE));
    }

    let v_split = split_vertex(mesh, v, v_prev, w, delta, dbg)?;
    let w_split = split_vertex(mesh, w, v, w_next, delta, false)?;
    let arc_to_dissolve = mesh.at_vertex(w_split).halfedge_to(v).try_end()?;
    dissolve_edge(mesh, arc_to_dissolve)?;

    let new_edge = mesh.at_vertex(v_split).halfedge_to(w_split).try_end()?;

    Ok(new_edge)
}
 */
