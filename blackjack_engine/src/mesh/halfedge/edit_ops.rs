use std::{
    collections::{BTreeMap, BTreeSet},
    f32::consts::PI,
};

use anyhow::{anyhow, bail};
use float_ord::FloatOrd;
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
        .outgoing_halfedges()?
        .iter()
        .map(|h| mesh.at_halfedge(*h).face().try_end())
        .collect::<Result<SVec<FaceId>, TraversalError>>()?
        .iter()
        .find(|f| mesh.face_vertices(**f).contains(&w))
        .cloned()
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
/// ## Id Stability
/// This operation guarantees that the outgoing halfedge ids are preserved.
/// Additionally, the returned vertex id vector has the newly created vertex ids
/// provided in the same order as `v`'s outgoing_halfedges
pub fn chamfer_vertex(
    mesh: &mut halfedge::MeshConnectivity,
    positions: &mut Positions,
    v: VertexId,
    interpolation_factor: f32,
) -> Result<(FaceId, SVec<VertexId>)> {
    let outgoing = mesh.at_vertex(v).outgoing_halfedges()?;
    let mut vertices = SVec::new();
    for &h in &outgoing {
        vertices.push(divide_edge(mesh, positions, h, interpolation_factor)?);
    }

    for (&v, &w) in vertices.iter().circular_tuple_windows() {
        cut_face(mesh, v, w)?;
    }

    Ok((dissolve_vertex(mesh, v)?, vertices))
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
/// TODO: This does not handle the case where a collapse edge operation would
/// remove a face
pub fn collapse_edge(mesh: &mut MeshConnectivity, h: HalfEdgeId) -> Result<VertexId> {
    let (v, w) = mesh.at_halfedge(h).src_dst_pair()?;
    let t = mesh.at_halfedge(h).twin().try_end()?;
    let h_next = mesh.at_halfedge(h).next().try_end()?;
    let h_prev = mesh.at_halfedge(h).previous().try_end()?;
    let t_next = mesh.at_halfedge(t).next().try_end()?;
    let t_prev = mesh.at_halfedge(t).previous().try_end()?;
    let w_outgoing = mesh.at_vertex(w).outgoing_halfedges()?;
    let v_next_fan = mesh.at_halfedge(h).cycle_around_fan().try_end()?;
    let f_h = mesh.at_halfedge(h).face().try_end();
    let f_t = mesh.at_halfedge(t).face().try_end();

    // --- Adjust connectivity ---
    for h_wo in w_outgoing {
        mesh[h_wo].vertex = Some(v);
    }
    mesh[t_prev].next = Some(t_next);
    mesh[h_prev].next = Some(h_next);

    // Some face may point to the halfedges we're deleting. Fix that.
    if let Ok(f_h) = f_h {
        if mesh.at_face(f_h).halfedge().try_end()? == h {
            mesh[f_h].halfedge = Some(h_next);
        }
    }
    if let Ok(f_t) = f_t {
        if mesh.at_face(f_t).halfedge().try_end()? == t {
            mesh[f_t].halfedge = Some(t_next);
        }
    }
    // The vertex we're keeping may be pointing to one of the deleted halfedges.
    if mesh.at_vertex(v).halfedge().try_end()? == h {
        mesh[v].halfedge = Some(v_next_fan);
    }

    // --- Remove data ----
    mesh.remove_halfedge(t);
    mesh.remove_halfedge(h);
    mesh.remove_vertex(w);

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
    for v in vertices_to_chamfer {
        let outgoing_halfedges = mesh.at_vertex(v).outgoing_halfedges()?;

        // After the chamfer operation, some vertex pairs need to get collapsed
        // into a single one. This binary vector has a `true` for every vertex
        // position where that needs to happen.
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

                h_b && h2_n || h_d && h2_b || h_d && h2_n || h_n && h2_b
            })
            .collect::<SVecN<_, 16>>();

        // Here, we execute the chamfer operation. The returned indices are
        // guaranteed to be in the same order as `v`'s outgoing halfedges.
        let (_, new_verts) = chamfer_vertex(mesh, positions, v, 0.0)?;

        let collapse_ops = new_verts
            .iter()
            .circular_tuple_windows()
            .zip(collapse_indices)
            .filter_map(|((v, w), should_collapse)| {
                if should_collapse {
                    // We want to keep w so next iterations don't produce dead
                    // vertex ids This is not entirely necessary since the
                    // translation map already ensures we will never access any
                    // dead vertices.
                    Some((*w, *v))
                } else {
                    None
                }
            })
            .collect::<SVecN<_, 16>>();

        // When collapsing vertices, we need a way to determine where those
        // original vertices ended up or we may access invalid ids
        type TranslationMap = HashMap<VertexId, VertexId>;
        let mut translation_map: TranslationMap = HashMap::new();
        /// Returns the translation of a vertex, that is, the vertex this vertex
        /// ended up being translated to.
        fn get_translated(m: &TranslationMap, v: VertexId) -> VertexId {
            let mut v = v;
            while let Some(v_tr) = m.get(&v) {
                v = *v_tr;
            }
            v
        }

        for (w, v) in collapse_ops {
            let v = get_translated(&translation_map, v);
            let w = get_translated(&translation_map, w);
            let h = mesh.at_vertex(w).halfedge_to(v).try_end()?;
            collapse_edge(mesh, h)?;
            translation_map.insert(v, w); // Take note that v is now w
        }
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
            }
        }
    }

    let beveled_edges = bevel_edges_connectivity(mesh, positions, &halfedges)?;

    // --- Adjust vertex positions ---

    // For each face, each vertex is pushed in the direction of the face's
    // normal vector. Vertices that share more than one face, get accumulated
    // pushes.
    let mut move_ops = HashMap::<VertexId, HashSet<Vec3Ord>>::new();
    for h in beveled_edges {
        // Find the halfedges adjacent to one of the extruded faces
        if mesh
            .at_halfedge(h)
            .face_or_boundary()?
            .map(|f| face_set.contains(&f))
            .unwrap_or(false)
        {
            let face = mesh.at_halfedge(h).face().try_end()?;
            let (src, dst) = mesh.at_halfedge(h).src_dst_pair()?;

            mesh.add_debug_halfedge(h, DebugMark::green("bvl"));

            let push = mesh
                .face_normal(positions, face)
                .ok_or_else(|| anyhow!("Attempted to extrude a face with only two vertices."))?
                * amount;

            move_ops
                .entry(src)
                .or_insert_with(HashSet::new)
                .insert(push.to_ord());
            move_ops
                .entry(dst)
                .or_insert_with(HashSet::new)
                .insert(push.to_ord());
        }
    }

    for (v_id, ops) in move_ops {
        positions[v_id] += ops.iter().fold(Vec3::ZERO, |x, y| x + y.to_vec());
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
        .chain(chain_2.iter().rev().tuple_windows())
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

    match (flip + 1) % 4 { // That +1 is experimentally determined to give nice results
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
        _ => unreachable!()
    }

    bridge_chains(mesh, &chain_1, &chain_2, is_closed)?;

    Ok(())
}

pub fn transform(
    mesh: &mut HalfEdgeMesh,
    translate: Vec3,
    rotate: Vec3,
    scale: Vec3,
) -> Result<()> {
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
pub fn add_edge(
    mesh: &mut HalfEdgeMesh,
    start: Vec3,
    end: Vec3,
) -> Result<(HalfEdgeId, HalfEdgeId)> {
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
