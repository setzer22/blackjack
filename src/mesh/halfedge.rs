use crate::prelude::*;

use generational_arena::Arena;
use glam::*;
use itertools::Itertools;
use smallvec::SmallVec;

/// Implements indexing traits so the mesh data structure can be used to access
/// vertex, face or halfedge information using ids as indices.
pub mod mesh_index_impls;

/// Type-safe wrappers over the internal allocator indices used as pointers
pub mod id_types;
pub use id_types::*;

/// An API to represent type-safe and error-handled graph traversals over a mesh
pub mod traversals;
pub use traversals::*;

/// Primitive shapes, like boxes or spheres
pub mod primitives;

/// High level polygon edit operations on a HalfEdge mesh like bevel, extrude
pub mod edit_ops;

/// Import / Export of HalfEdgeMesh data structure to Wavefront OBJ files
pub mod wavefront_obj;

/// HalfEdge meshes are a type of linked list. This means it is sometimes
/// impossible to ensure some algorithms will terminate when the mesh is
/// malformed. To ensure the code never goes into an infinite loop, this max
/// number of iterations will be performed before giving an error. This error
/// should be large enough, as faces with a very large number of vertices may
/// trigger it.
const MAX_LOOP_ITERATIONS: usize = 32;

#[derive(Debug, Default, Clone)]
pub struct HalfEdge {
    twin: Option<HalfEdgeId>,
    next: Option<HalfEdgeId>,
    vertex: Option<VertexId>,
    face: Option<FaceId>,
}

#[derive(Debug, Clone)]
pub struct Vertex {
    pub position: Vec3,
    halfedge: Option<HalfEdgeId>,
}

#[derive(Debug, Clone)]
pub struct Face {
    halfedge: Option<HalfEdgeId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DebugMark {
    pub label: String,
    pub color: egui::Color32,
}

impl DebugMark {
    pub fn blue(label: &str) -> Self {
        Self::new(label, egui::Color32::BLUE)
    }

    pub fn red(label: &str) -> Self {
        Self::new(label, egui::Color32::RED)
    }

    pub fn green(label: &str) -> Self {
        Self::new(label, egui::Color32::GREEN)
    }

    pub fn new(label: &str, color: egui::Color32) -> Self {
        Self {
            label: label.to_owned(),
            color,
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct HalfEdgeMesh {
    vertices: Arena<Vertex>,
    faces: Arena<Face>,
    halfedges: Arena<HalfEdge>,

    debug_edges: HashMap<HalfEdgeId, DebugMark>,
    debug_vertices: HashMap<VertexId, DebugMark>,
}

pub type SVec<T> = SmallVec<[T; 4]>;
pub type SVecN<T, const N: usize> = SmallVec<[T; N]>;

impl HalfEdgeMesh {
    // Adds a disconnected quad into the mesh. Returns the id to the first
    // halfedge of the quad
    pub fn add_quad(&mut self, a: Vec3, b: Vec3, c: Vec3, d: Vec3) -> HalfEdgeId {
        let v_a = self.alloc_vertex(a, None);
        let v_b = self.alloc_vertex(b, None);
        let v_c = self.alloc_vertex(c, None);
        let v_d = self.alloc_vertex(d, None);

        let f = self.alloc_face(None);

        let h_a_b = self.alloc_halfedge(HalfEdge {
            vertex: Some(v_a),
            face: Some(f),
            ..Default::default()
        });
        let h_b_c = self.alloc_halfedge(HalfEdge {
            vertex: Some(v_b),
            face: Some(f),
            ..Default::default()
        });
        let h_c_d = self.alloc_halfedge(HalfEdge {
            vertex: Some(v_c),
            face: Some(f),
            ..Default::default()
        });
        let h_d_a = self.alloc_halfedge(HalfEdge {
            vertex: Some(v_d),
            face: Some(f),
            ..Default::default()
        });

        // Make the half-edge loop
        self[h_a_b].next = Some(h_b_c);
        self[h_b_c].next = Some(h_c_d);
        self[h_c_d].next = Some(h_d_a);
        self[h_d_a].next = Some(h_a_b);

        // Set the face for all half-edges
        let half_edges = [h_a_b, h_b_c, h_c_d, h_d_a];
        for h in half_edges {
            self[h].face = Some(f);
        }

        // Set the half-edges for the face and vertices
        self[f].halfedge = Some(h_a_b);
        self[v_a].halfedge = Some(h_a_b);
        self[v_b].halfedge = Some(h_b_c);
        self[v_c].halfedge = Some(h_c_d);
        self[v_d].halfedge = Some(h_d_a);

        h_a_b
    }

    /// Returns the number of edges a face has
    pub fn num_face_edges(&self, face_id: FaceId) -> usize {
        self.face_edges(face_id).len()
    }

    /// Returns the edges of a given face
    pub fn face_edges(&self, face_id: FaceId) -> SVec<HalfEdgeId> {
        let mut edges = SmallVec::new();
        let h0 = self[face_id].halfedge.expect("Face should have a halfedge");
        let mut h = h0;

        edges.push(h);

        let mut counter = 0;

        loop {
            if counter > MAX_LOOP_ITERATIONS {
                panic!("Max number of iterations reached. Is the mesh malformed?");
            }
            counter += 1;

            h = self[h]
                .next
                .unwrap_or_else(|| panic!("Halfedge {:?} has no next", h));
            if h == h0 {
                break;
            }
            edges.push(h);
        }

        edges
    }

    pub fn face_vertices(&self, face_id: FaceId) -> SVec<VertexId> {
        self.face_edges(face_id)
            .iter()
            .map(|e| self.at_halfedge(*e).vertex().end())
            .collect()
    }

    pub fn generate_buffers(&self) -> (Vec<Vec3>, Vec<u32>) {
        let mut done_faces: HashSet<FaceId> = HashSet::new();

        let mut positions = vec![];
        let mut indices = vec![];
        let mut next_index = 0;

        for (face_idx, _face) in self.faces.iter() {
            let face_id = FaceId(face_idx);
            if done_faces.contains(&face_id) {
                continue;
            }
            done_faces.insert(face_id);

            let vertices = self.face_vertices(face_id);

            let v1 = vertices[0];

            for (&v2, &v3) in vertices[1..].iter().tuple_windows() {
                let v1_pos = self[v1].position;
                let v2_pos = self[v2].position;
                let v3_pos = self[v3].position;

                positions.push(v1_pos);
                positions.push(v2_pos);
                positions.push(v3_pos);
                indices.push(next_index);
                indices.push(next_index + 1);
                indices.push(next_index + 2);
                next_index += 3;
            }
        }

        (positions, indices)
    }

    pub fn edge_endpoints(&mut self, edge: HalfEdgeId) -> (VertexId, VertexId) {
        let a = self.at_halfedge(edge).vertex().end();
        let b = self.at_halfedge(edge).next().vertex().end();
        (a, b)
    }

    pub fn extrude_edge(&mut self, edge: HalfEdgeId, a_to: Vec3, b_to: Vec3) -> Result<HalfEdgeId> {
        if self[edge].twin.is_some() {
            bail!("Attempt to extrude an edge that already has a twin. Would result in a non-manifold mesh.")
        }
        let (a, b) = self.edge_endpoints(edge);
        let f = self.alloc_face(None);
        let a2 = self.alloc_vertex(a_to, None);
        let b2 = self.alloc_vertex(b_to, None);

        let h1 = self.alloc_halfedge(HalfEdge {
            twin: None,
            next: None,
            vertex: Some(a),
            face: Some(f),
        });
        let h2 = self.alloc_halfedge(HalfEdge {
            twin: None,
            next: None,
            vertex: Some(a2),
            face: Some(f),
        });
        let h3 = self.alloc_halfedge(HalfEdge {
            twin: None,
            next: None,
            vertex: Some(b2),
            face: Some(f),
        });
        let h4 = self.alloc_halfedge(HalfEdge {
            twin: None,
            next: None,
            vertex: Some(b),
            face: Some(f),
        });

        self[h1].next = Some(h2);
        self[h2].next = Some(h3);
        self[h3].next = Some(h4);
        self[h4].next = Some(h1);

        self[h4].twin = Some(edge);

        self[f].halfedge = Some(h1);
        self[a2].halfedge = Some(h2);
        self[a2].halfedge = Some(h3);

        Ok(h2)
    }

    /// Builds this mesh from a list of vertices, and a list of polygons,
    /// containing indices that reference those vertices.
    ///
    /// - Generic over Index: Use as much precision as you need / want.
    /// - Generic over Polygon: Use whatever input layout you want.
    ///
    /// If unsure, you can pass `Vec<Vec<u32>>` as `polygons`. You can also use
    /// `[[u32;3]]` or `&[&[u32]]`. Same for `u8`, `u16` or `usize` indices.
    pub fn build_from_polygons<'a, Index, Polygon>(
        positions: &[Vec3],
        polygons: &[Polygon],
    ) -> Result<Self>
    where
        Index: Into<usize> + 'static + Eq + PartialEq + core::hash::Hash + Copy,
        Polygon: AsRef<[Index]>,
    {
        let mut mesh = Self::default();

        // Maps indices from the `polygons` array to the allocated vertices in
        // the newly created halfedge mesh.
        let mut index_to_vertex = HashMap::<Index, VertexId>::new();

        // Used to compute the degree of a vertex. Useful to do some sanity
        // checks.
        let mut vertex_degree = HashMap::<VertexId, u32>::new();

        // First pass over polygon data to determine some initial properties
        for polygon in polygons.iter().map(|p| p.as_ref()) {
            // Some sanity checks
            if polygon.len() < 3 {
                bail!("Cannot build meshes where polygons have less than three vertices.")
            }
            if polygon.iter().duplicates().next().is_some() {
                bail!("Cannot not build meshes where a polygon has duplicate vertices")
            }

            // Compute correspondence between vertices and indices. Also fill in vertex degree data.
            for index in polygon {
                // Create the vertex if it doesn't exist
                let idx = Into::<usize>::into(*index); // ugh
                let position = positions
                    .get(idx)
                    .ok_or_else(|| anyhow!("Out-of-bounds index in the polygon array {}", idx))?;
                let v_id = index_to_vertex
                    .entry(*index)
                    .or_insert_with(|| mesh.alloc_vertex(*position, None));

                // Increment the vertex degree counter for that vertex.
                *vertex_degree.entry(*v_id).or_insert(0) += 1;
            }
        }

        // After the sanity checks, we know the amount of vertices and faces.
        let _num_vertices = index_to_vertex.len();
        let _num_faces = polygons.len();

        // Maps pairs of indices to mesh halfedges
        let mut pair_to_halfedge = HashMap::<(Index, Index), HalfEdgeId>::new();

        // We can now start building connectivity information by doing a second
        // pass over the polygon list
        for polygon in polygons.iter().map(|p| p.as_ref()) {
            // Cyclically ordered list of the half edge ids of this face.
            let mut half_edges_in_face = SVec::new();

            let face = mesh.alloc_face(None);

            for (&a, &b) in polygon.iter().circular_tuple_windows() {
                if pair_to_halfedge.get(&(a, b)).is_some() {
                    bail!(
                        "Found multiple oriented edges with the same indices.\
                         This means either (i) surface is non-manifold or (ii) faces \
                         are not oriented in the same direction"
                    )
                }

                let h = mesh.alloc_halfedge(HalfEdge::default());
                // Link halfedge to face
                mesh[h].face = Some(face);
                mesh[face].halfedge = Some(h);

                // Link halfedge to source vertex
                let v_a = index_to_vertex[&a];
                mesh[h].vertex = Some(v_a);
                mesh[v_a].halfedge = Some(h);

                half_edges_in_face.push(h);

                pair_to_halfedge.insert((a, b), h);

                if let Some(&other) = pair_to_halfedge.get(&(b, a)) {
                    mesh[h].twin = Some(other);
                    mesh[other].twin = Some(h);
                }
            }

            for (&h1, &h2) in half_edges_in_face.iter().circular_tuple_windows() {
                mesh[h1].next = Some(h2);
            }
        }

        // Make vertices in the boundary point to a halfedge that is also on the
        // boundary.
        //
        // NOTE: @setzer22 The original code did this, but it didn't explain why
        // and it's not immediately obvious.

        // NOTE: We need to use this vector to defer the actual mutation because
        // we can't iterate the mesh and do queries at the same time. It could
        // be optimized by separating the generational arenas as separate
        // borrows and using those. But the loss in clarity is not worth it.
        let mut defer_vertex_halfedge_replacement = vec![];

        for (v_id, vertex) in mesh.iter_vertices() {
            let h0 = vertex.halfedge.expect("Should have halfedge by now");
            let mut h = h0;

            let mut counter = 0;

            loop {
                if counter > MAX_LOOP_ITERATIONS {
                    panic!("Max number of iterations reached. Is the mesh malformed?");
                }
                counter += 1;

                if mesh[h].twin.is_none() {
                    defer_vertex_halfedge_replacement.push((v_id, h));
                    break;
                }
                h = mesh.at_halfedge(h).twin().next().end();
                if h == h0 {
                    break;
                }
            }
        }
        for (v, h) in defer_vertex_halfedge_replacement {
            mesh[v].halfedge = Some(h);
        }

        // Construct the boundary halfedges. Right now, the boundary consists of
        // incomplete edges, i.e. half edges that do not have a twin. Leaving it
        // like this would complicate some kinds of traversal because we can't
        // rely on halfedges always having a twin. We will instead create
        // boundary half edges: That is, twin halfedges that do not point to any
        // face. The boundary halfedges are linked following a circle around the
        // closed boundary. It's easier to imagine this by thinking of a hole in
        // the mesh, but it works just as well if you think about the "outside"
        // of a quad grid as a hole, as the loop would go all around the quad

        // Clone to avoid double-borrow issues
        // TODO: Again, this could be optimized. Don't care for now.
        let halfedges: Vec<HalfEdgeId> = mesh.iter_halfedges().map(|(h, _)| h).collect();

        for &h0 in halfedges.iter() {
            let mut boundary_halfedges = Vec::<HalfEdgeId>::new();
            if mesh[h0].twin.is_none() {
                let mut h_it = h0;
                loop {
                    let t = mesh.alloc_halfedge(HalfEdge::default());
                    boundary_halfedges.push(t);
                    mesh[h_it].twin = Some(t);
                    mesh[t].twin = Some(h_it);
                    mesh[t].vertex = Some(mesh.at_halfedge(h_it).next().vertex().end());

                    // Look for the next outgoing halfedge for this vertex
                    // that's in the boundary
                    h_it = mesh.at_halfedge(h_it).next().end();
                    while h_it != h0 && mesh[h_it].twin.is_some() {
                        // Twin-next cycles around the outgoing halfedges of a vertex
                        h_it = mesh.at_halfedge(h_it).twin().next().end();
                    }

                    if h_it == h0 {
                        break;
                    }
                }
            }

            for (&b_h, &b_h_next) in boundary_halfedges.iter().rev().circular_tuple_windows() {
                mesh[b_h].next = Some(b_h_next);
            }
        }

        // Cycle the halfedge pointers for vertices again. Original code says it
        // makes this to make "traversal easier" :shrug:
        let vertices: Vec<VertexId> = mesh.iter_vertices().map(|(v, _)| v).collect(); // Yet another spurious copy.
        for v in vertices {
            mesh[v].halfedge = Some(mesh.at_vertex(v).halfedge().twin().next().end());
        }

        // Do some final manifoldness checks
        for (v, vertex) in mesh.iter_vertices() {
            if vertex.halfedge.is_none() {
                bail!("There is at least a single vertex that's disconnected from any polygon");
            }

            // Check that the number of halfedges emanating from this vertex
            // equal the number of polygons containing this vertex. If this
            // doesn't check out, it means our vertex is not a polygon "fan",
            // but some other (thus, non-manifold) structure
            let h0 = mesh.at_vertex(v).halfedge().end();
            let mut h = h0;
            let mut count = 0;
            loop {
                if !mesh.at_halfedge(h).is_boundary().unwrap() {
                    count += 1;
                }
                h = mesh.at_halfedge(h).twin().next().end();

                if h == h0 {
                    break;
                }
            }

            if count != vertex_degree[&v] {
                bail!("At least one of the vertices is not a polygon fan, but some other nonmanifold structure instead.")
            }
        }

        Ok(mesh)
    }

    /// Reverses the direction of the halfedges in a face.
    /// NOTE: This breaks manifoldness. Do not do it unless you know what you're doing.
    fn flip_face(&mut self, face_id: FaceId) {
        let edges = self.face_edges(face_id);
        for (&e1, &e2) in edges.iter().rev().circular_tuple_windows() {
            self[e1].next = Some(e2);
        }
    }

    fn halfedge_loop(&self, h0: HalfEdgeId) -> SVec<HalfEdgeId> {
        let mut ret = smallvec::smallvec![h0];
        let mut h = h0;

        let mut count = 0;

        loop {
            if count > MAX_LOOP_ITERATIONS {
                panic!("Max number of iterations reached. Is the mesh malformed?");
            }
            count += 1;

            h = self[h].next.expect("Halfedges should form a loop");
            if h == h0 {
                break;
            } else {
                ret.push(h);
            }
        }
        ret
    }

    fn previous_halfedge(&self, h: HalfEdgeId) -> HalfEdgeId {
        *self
            .halfedge_loop(h)
            .last()
            .expect("Halfedge loop must always return a positive size vector")
    }

    pub fn iter_vertices(&self) -> impl Iterator<Item = (VertexId, &Vertex)> {
        self.vertices.iter().map(|(idx, v)| (VertexId(idx), v))
    }

    pub fn iter_faces(&self) -> impl Iterator<Item = (FaceId, &Face)> {
        self.faces.iter().map(|(idx, f)| (FaceId(idx), f))
    }

    pub fn iter_halfedges(&self) -> impl Iterator<Item = (HalfEdgeId, &HalfEdge)> {
        self.halfedges.iter().map(|(idx, h)| (HalfEdgeId(idx), h))
    }

    /// Sets the position for a given vertex
    pub fn set_vertex_position(&mut self, vertex: VertexId, position: Vec3) {
        self.vertex_mut(vertex).unwrap().position = position;
    }

    /// Sets the position for a given vertex
    pub fn vertex_position(&self, vertex: VertexId) -> Vec3 {
        self.vertex(vertex).unwrap().position
    }

    /// Sets the position for a given vertex using updater function. Function takes old value.
    pub fn update_vertex_position(&mut self, vertex: VertexId, updater: impl FnOnce(Vec3) -> Vec3) {
        let v = self.vertex_mut(vertex).unwrap();
        v.position = updater(v.position);
    }

    /// Adds a new vertex to the mesh, disconnected from everything else. Returns its handle.
    fn alloc_vertex(&mut self, position: Vec3, halfedge: Option<HalfEdgeId>) -> VertexId {
        VertexId(self.vertices.insert(Vertex { position, halfedge }))
    }

    /// Adds a new face to the mesh, disconnected from everything else. Returns its handle.
    fn alloc_face(&mut self, halfedge: Option<HalfEdgeId>) -> FaceId {
        FaceId(self.faces.insert(Face { halfedge }))
    }

    /// Removes a face from the mesh. This does not attempt to preserve mesh
    /// connectivity and should only be used as part of internal operations.
    fn remove_face(&mut self, face: FaceId) {
        self.faces.remove(face.0);
    }

    /// Removes a halfedge from the mesh. This does not attempt to preserve mesh
    /// connectivity and should only be used as part of internal operations.
    fn remove_halfedge(&mut self, halfedge: HalfEdgeId) {
        self.halfedges.remove(halfedge.0);
        self.debug_edges.remove(&halfedge);
    }

    /// Removes a vertex from the mesh. This does not attempt to preserve mesh
    /// connectivity and should only be used as part of internal operations.
    fn remove_vertex(&mut self, vertex: VertexId) {
        self.vertices.remove(vertex.0);
        self.debug_vertices.remove(&vertex);
    }

    /// Adds a new vertex to the mesh, disconnected from everything else. Returns its handle.
    fn alloc_halfedge(&mut self, halfedge: HalfEdge) -> HalfEdgeId {
        HalfEdgeId(self.halfedges.insert(halfedge))
    }

    pub fn vertex_debug_mark(&self, vertex: VertexId) -> Option<DebugMark> {
        self.debug_vertices.get(&vertex).cloned()
    }

    pub fn add_debug_vertex(&mut self, vertex: VertexId, mark: DebugMark) {
        self.debug_vertices.insert(vertex, mark);
    }

    pub fn halfedge_debug_mark(&self, edge: HalfEdgeId) -> Option<DebugMark> {
        self.debug_edges.get(&edge).cloned()
    }

    pub fn add_debug_halfedge(&mut self, h: HalfEdgeId, mark: DebugMark) {
        self.debug_edges.insert(h, mark);
    }

    pub fn iter_debug_halfedges(&self) -> impl Iterator<Item = (&HalfEdgeId, &DebugMark)> {
        self.debug_edges.iter()
    }

    pub fn iter_debug_vertices(&self) -> impl Iterator<Item = (&VertexId, &DebugMark)> {
        self.debug_vertices.iter()
    }

    pub fn clear_debug(&mut self) {
        self.debug_edges.clear();
        self.debug_vertices.clear();
    }

    /// Returns the average of a face's vertices. Note that this is different
    /// from the centroid. See:
    /// https://en.wikipedia.org/wiki/Centroid#Of_a_polygon
    /// https://stackoverflow.com/questions/2355931/compute-the-centroid-of-a-3d-planar-polygon
    pub fn face_vertex_average(&self, face_id: FaceId) -> Vec3 {
        let face_vertices = self
            .face_vertices(face_id)
            .iter()
            .map(|v| self.vertex_position(*v))
            .collect::<SVec<_>>();
        face_vertices.iter().fold(Vec3::ZERO, |v1, v2| v1 + *v2) / face_vertices.len() as f32
    }

    pub fn vertex_exists(&self, vertex: VertexId) -> bool {
        self.vertex(vertex).is_some()
    }

    /// Merges this halfedge mesh with another one. No additional connectivity
    /// data is generated between the two.
    pub fn merge_with(&mut self, mesh_b: &HalfEdgeMesh) {
        let mut vmap = HashMap::<VertexId, VertexId>::new();
        let mut hmap = HashMap::<HalfEdgeId, HalfEdgeId>::new();
        let mut fmap = HashMap::<FaceId, FaceId>::new();

        // On a first pass, we reserve new vertices, faces and halfedges without
        // setting any of their pointers and store their ids in a mapping.
        for (vertex_id, vertex) in mesh_b.iter_vertices() {
            vmap.insert(vertex_id, self.alloc_vertex(vertex.position, None));
        }
        for (face_id, _) in mesh_b.iter_faces() {
            fmap.insert(face_id, self.alloc_face(None));
        }
        for (halfedge_id, _) in mesh_b.iter_halfedges() {
            hmap.insert(
                halfedge_id,
                self.alloc_halfedge(HalfEdge {
                    twin: None,
                    next: None,
                    vertex: None,
                    face: None,
                }),
            );
        }

        // The second pass uses the mapping and the original data to set all the
        // inner pointers.
        for (vertex_id, vertex) in mesh_b.iter_vertices() {
            if let Some(h) = vertex.halfedge {
                self[vmap[&vertex_id]].halfedge = Some(hmap[&h])
            }
        }
        for (face_id, face) in mesh_b.iter_faces() {
            if let Some(h) = face.halfedge {
                self[fmap[&face_id]].halfedge = Some(hmap[&h])
            }
        }
        for (halfedge_id, halfedge) in mesh_b.iter_halfedges() {
            if let Some(twin) = halfedge.twin {
                self[hmap[&halfedge_id]].twin = Some(hmap[&twin]);
            }
            if let Some(next) = halfedge.next {
                self[hmap[&halfedge_id]].next = Some(hmap[&next]);
            }
            if let Some(vertex) = halfedge.vertex {
                self[hmap[&halfedge_id]].vertex = Some(vmap[&vertex]);
            }
            if let Some(face) = halfedge.face {
                self[hmap[&halfedge_id]].face = Some(fmap[&face]);
            }
        }
    }

    // Returns the normal of the face. The first three vertices are used to
    // compute the normal. If the vertices of the face are not coplanar,
    // the result will not be correct.
    fn face_normal(&self, face: FaceId) -> Vec3 {
        let verts = self.face_vertices(face);
        // Will panic if face has two or less vertices. Note that faces with two
        // vertices are possible (they get generated as part of the bevel
        // operation). But this would only fail if the operation is used on one
        // such face in the middle of an operation, not in normal operation.
        let v01 = self.vertex_position(verts[0]) - self.vertex_position(verts[1]);
        let v12 = self.vertex_position(verts[1]) - self.vertex_position(verts[2]);

        v01.cross(v12).normalize()
    }
}

#[cfg(test)]
pub mod test {
    use super::*;

    fn quad_abcd() -> (Vec3, Vec3, Vec3, Vec3) {
        (
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 1.0),
            Vec3::new(0.0, 0.0, 1.0),
        )
    }

    #[test]
    pub fn test_add_quad() {
        let mut hem = HalfEdgeMesh::default();
        let (a, b, c, d) = quad_abcd();
        let q = hem.add_quad(a, b, c, d);

        assert_eq!(hem.at_halfedge(q).next().next().next().next().end(), q);

        assert_eq!(hem[hem.at_halfedge(q).vertex().end()].position, a);
        assert_eq!(hem[hem.at_halfedge(q).next().vertex().end()].position, b,);
        assert_eq!(
            hem[hem.at_halfedge(q).next().next().vertex().end()].position,
            c,
        );
        assert_eq!(
            hem[hem.at_halfedge(q).next().next().next().vertex().end()].position,
            d,
        );

        assert_eq!(
            hem.at_halfedge(q).face().end(),
            hem.at_halfedge(q).next().face().end()
        );
    }

    #[test]
    pub fn test_face_size() {
        let mut hem = HalfEdgeMesh::default();
        let (a, b, c, d) = quad_abcd();
        let q = hem.add_quad(a, b, c, d);

        let f = hem.at_halfedge(q).face().end();
        assert_eq!(hem.num_face_edges(f), 4);
    }

    #[test]
    pub fn generate_quad_buffers() {
        let mut hem = HalfEdgeMesh::default();
        let (a, b, c, d) = quad_abcd();
        let _q = hem.add_quad(a, b, c, d);

        dbg!(hem.generate_buffers());
    }
}
