// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{marker::PhantomData, rc::Rc};

use crate::{
    prelude::*,
    sync::{BorrowedRef, InteriorMutable, MutableRef},
};

use glam::*;
use itertools::Itertools;
use slotmap::{SecondaryMap, SlotMap};
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

/// A compact halfedge graph specifically optimized for some operations
pub mod compact_mesh;

/// Types to represent a selection of a subset of faces, vertices or edges.
pub mod selection;

/// Generate vertex and index buffers suitable to be uploaded to the GPU for rendering
pub mod gpu_buffer_generation;
pub use gpu_buffer_generation::*;

pub mod halfedge_lua_api;

pub mod channels;
pub use channels::*;

use self::mappings::MeshMapping;

/// HalfEdge meshes are a type of linked list. This means it is sometimes
/// impossible to ensure some algorithms will terminate when the mesh is
/// malformed. To ensure the code never goes into an infinite loop, this max
/// number of iterations will be performed before giving an error. This error
/// should be large enough, as faces with a very large number of vertices may
/// trigger it.
pub const MAX_LOOP_ITERATIONS: usize = 8196;

#[derive(Debug, Default, Clone)]
pub struct HalfEdge {
    twin: Option<HalfEdgeId>,
    next: Option<HalfEdgeId>,
    vertex: Option<VertexId>,
    face: Option<FaceId>,
}

#[derive(Debug, Clone)]
pub struct Vertex {
    halfedge: Option<HalfEdgeId>,
}

#[derive(Debug, Clone)]
pub struct Face {
    halfedge: Option<HalfEdgeId>,
}

/// Stored as RGBA
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DebugColor(pub u32);

impl DebugColor {
    pub fn r(&self) -> u8 {
        (self.0 >> 24) as u8
    }

    pub fn g(&self) -> u8 {
        ((self.0 >> 16) & 0x00_00_00_ff) as u8
    }

    pub fn b(&self) -> u8 {
        ((self.0 >> 8) & 0x00_00_00_ff) as u8
    }

    pub fn a(&self) -> u8 {
        (self.0 & 0x00_00_00_ff) as u8
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DebugMark {
    pub label: String,
    pub color: DebugColor,
}

impl DebugMark {
    pub fn blue(label: &str) -> Self {
        Self::new(label, DebugColor(0x00_00_ff_ff))
    }

    pub fn red(label: &str) -> Self {
        Self::new(label, DebugColor(0xff_00_00_ff))
    }

    pub fn green(label: &str) -> Self {
        Self::new(label, DebugColor(0x00_ff_00_ff))
    }

    pub fn purple(label: &str) -> Self {
        Self::new(label, DebugColor(0xff_00_ff_ff))
    }

    pub fn new(label: &str, color: DebugColor) -> Self {
        Self {
            label: label.to_owned(),
            color,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MeshConnectivity {
    vertices: SlotMap<VertexId, Vertex>,
    faces: SlotMap<FaceId, Face>,
    halfedges: SlotMap<HalfEdgeId, HalfEdge>,

    debug_edges: HashMap<HalfEdgeId, DebugMark>,
    debug_vertices: HashMap<VertexId, DebugMark>,
}

/// This struct contains some parameters that allow configuring the way in which
/// a mesh is generated.
#[derive(Default, Debug, Clone)]
pub struct MeshGenerationConfig {
    /// Should this mesh be generated using smooth (i.e. per-vertex) normals? Or
    /// flat (i.e. per-face) normals?
    pub smooth_normals: bool,
}

#[derive(Debug)]
#[cfg_attr(not(feature = "sync"), derive(Clone))]
pub struct HalfEdgeMesh {
    connectivity: InteriorMutable<MeshConnectivity>,
    pub channels: MeshChannels,
    default_channels: DefaultChannels,
    pub gen_config: MeshGenerationConfig,
}

#[cfg(feature = "sync")]
impl Clone for HalfEdgeMesh {
    fn clone(&self) -> Self {
        HalfEdgeMesh {
            connectivity: InteriorMutable::new(self.connectivity.borrow().clone()),
            channels: self.channels.clone(),
            default_channels: self.default_channels.clone(),
            gen_config: self.gen_config.clone(),
        }
    }
}

pub type Positions = Channel<VertexId, Vec3>;

impl MeshConnectivity {
    pub fn new() -> Self {
        Self::default()
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
                .unwrap_or_else(|| panic!("Halfedge {h:?} has no next"));
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

    pub fn edge_endpoints(&mut self, edge: HalfEdgeId) -> (VertexId, VertexId) {
        let a = self.at_halfedge(edge).vertex().end();
        let b = self.at_halfedge(edge).next().vertex().end();
        (a, b)
    }

    pub fn extrude_edge(
        &mut self,
        positions: &mut Positions,
        edge: HalfEdgeId,
        a_to: Vec3,
        b_to: Vec3,
    ) -> Result<HalfEdgeId> {
        if self[edge].twin.is_some() {
            bail!("Attempt to extrude an edge that already has a twin. Would result in a non-manifold mesh.")
        }
        let (a, b) = self.edge_endpoints(edge);
        let f = self.alloc_face(None);
        let a2 = self.alloc_vertex(positions, a_to, None);
        let b2 = self.alloc_vertex(positions, b_to, None);

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

    /// Given a `self` in an inconsistent state, where some halfedges have no
    /// `twin` (because it's in the boundary), this method adds twin halfedges
    /// forming a loop across the boundaries of the mesh. The new halfedges will
    /// be marked as boundary with a None face.
    fn add_boundary_halfedges(&mut self) {
        // Clone to avoid double-borrow issues
        // TODO: Again, this could be optimized. Don't care for now.
        let halfedges: Vec<HalfEdgeId> = self.iter_halfedges().map(|(h, _)| h).collect();

        for &h0 in halfedges.iter() {
            let mut boundary_halfedges = Vec::<HalfEdgeId>::new();
            if self[h0].twin.is_none() {
                let mut h_it = h0;
                loop {
                    let t = self.alloc_halfedge(HalfEdge::default());
                    boundary_halfedges.push(t);
                    self[h_it].twin = Some(t);
                    self[t].twin = Some(h_it);
                    self[t].vertex = Some(self.at_halfedge(h_it).next().vertex().end());

                    // Look for the next outgoing halfedge for this vertex
                    // that's in the boundary
                    h_it = self.at_halfedge(h_it).next().end();
                    while h_it != h0 && self[h_it].twin.is_some() {
                        // Twin-next cycles around the outgoing halfedges of a vertex
                        h_it = self.at_halfedge(h_it).twin().next().end();
                    }

                    if h_it == h0 {
                        break;
                    }
                }
            }

            for (&b_h, &b_h_next) in boundary_halfedges.iter().rev().circular_tuple_windows() {
                self[b_h].next = Some(b_h_next);
            }
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

    /// Returns an iterator that follows the next pointer for halfedges starting
    /// at `h0` until closing the loop.
    fn halfedge_loop_iter(&self, h0: HalfEdgeId) -> HalfedgeOpIterator<'_, NextOp> {
        HalfedgeOpIterator {
            conn: self,
            start: h0,
            next: h0,
            count: 0,
            _op: PhantomData,
        }
    }

    /// Returns an iterator that cycles around the halfedge fan starting at `h0`
    /// until closing the loop.
    #[allow(dead_code)]
    fn halfedge_fan_iter(&self, h0: HalfEdgeId) -> HalfedgeOpIterator<'_, CycleFanOp> {
        HalfedgeOpIterator {
            conn: self,
            start: h0,
            next: h0,
            count: 0,
            _op: PhantomData,
        }
    }

    pub fn iter_vertices(&self) -> impl Iterator<Item = (VertexId, &Vertex)> {
        self.vertices.iter()
    }

    pub fn iter_vertices_with_channel<'a, T: ChannelValue>(
        &'a self,
        channel: &'a Channel<VertexId, T>,
    ) -> impl Iterator<Item = (VertexId, &Vertex, T)> + 'a {
        self.vertices.iter().map(|(id, v)| (id, v, channel[id]))
    }

    pub fn iter_faces(&self) -> impl Iterator<Item = (FaceId, &Face)> {
        self.faces.iter()
    }

    pub fn iter_faces_with_channel<'a, T: ChannelValue>(
        &'a self,
        channel: &'a Channel<FaceId, T>,
    ) -> impl Iterator<Item = (FaceId, &Face, T)> + 'a {
        self.faces.iter().map(|(id, v)| (id, v, channel[id]))
    }

    pub fn iter_halfedges(&self) -> impl Iterator<Item = (HalfEdgeId, &HalfEdge)> {
        self.halfedges.iter()
    }

    pub fn iter_halfedges_with_channel<'a, T: ChannelValue>(
        &'a self,
        channel: &'a Channel<HalfEdgeId, T>,
    ) -> impl Iterator<Item = (HalfEdgeId, &HalfEdge, T)> + 'a {
        self.halfedges.iter().map(|(id, v)| (id, v, channel[id]))
    }

    /// Adds a new vertex to the mesh, disconnected from everything else. Returns its handle.
    fn alloc_vertex(
        &mut self,
        positions: &mut Positions,
        position: Vec3,
        halfedge: Option<HalfEdgeId>,
    ) -> VertexId {
        let v = self.vertices.insert(Vertex { halfedge });
        positions[v] = position;
        v
    }

    /// Adds a new vertex to the mesh, disconnected from everything else.
    /// Returns its handle. Unlike `alloc_vertex`, this function does not set
    /// the vertex position, implicitly leaving it at zero.
    fn alloc_vertex_raw(&mut self, halfedge: Option<HalfEdgeId>) -> VertexId {
        self.vertices.insert(Vertex { halfedge })
    }

    /// Adds a new face to the mesh, disconnected from everything else. Returns its handle.
    fn alloc_face(&mut self, halfedge: Option<HalfEdgeId>) -> FaceId {
        self.faces.insert(Face { halfedge })
    }

    /// Removes a face from the mesh. This does not attempt to preserve mesh
    /// connectivity and should only be used as part of internal operations.
    fn remove_face(&mut self, face: FaceId) {
        self.faces.remove(face);
    }

    /// Removes a halfedge from the mesh. This does not attempt to preserve mesh
    /// connectivity and should only be used as part of internal operations.
    fn remove_halfedge(&mut self, halfedge: HalfEdgeId) {
        self.halfedges.remove(halfedge);
        self.debug_edges.remove(&halfedge);
    }

    /// Removes a vertex from the mesh. This does not attempt to preserve mesh
    /// connectivity and should only be used as part of internal operations.
    fn remove_vertex(&mut self, vertex: VertexId) {
        self.vertices.remove(vertex);
        self.debug_vertices.remove(&vertex);
    }

    /// Adds a new vertex to the mesh, disconnected from everything else. Returns its handle.
    fn alloc_halfedge(&mut self, halfedge: HalfEdge) -> HalfEdgeId {
        self.halfedges.insert(halfedge)
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
    pub fn face_vertex_average(&self, positions: &Positions, face_id: FaceId) -> Vec3 {
        let face_vertices = self
            .face_vertices(face_id)
            .iter()
            .map(|v| positions[*v])
            .collect::<SVec<_>>();
        face_vertices.iter().fold(Vec3::ZERO, |v1, v2| v1 + *v2) / face_vertices.len() as f32
    }

    pub fn vertex_exists(&self, vertex: VertexId) -> bool {
        self.vertex(vertex).is_some()
    }

    // Returns the normal of the face. The first three vertices are used to
    // compute the normal. If the vertices of the face are not coplanar,
    // the result will not be correct.
    fn face_normal(&self, positions: &Positions, face: FaceId) -> Option<Vec3> {
        let verts = self.face_vertices(face);
        if verts.len() >= 3 {
            let v01 = positions[verts[0]] - positions[verts[1]];
            let v12 = positions[verts[1]] - positions[verts[2]];
            Some(v01.cross(v12).normalize())
        } else {
            None
        }
    }

    pub fn num_halfedges(&self) -> usize {
        self.halfedges.len()
    }

    pub fn num_vertices(&self) -> usize {
        self.vertices.len()
    }

    pub fn num_faces(&self) -> usize {
        self.faces.len()
    }
}

impl HalfEdgeMesh {
    pub fn new() -> Self {
        let mut channels = MeshChannels::default();
        let default_channels = DefaultChannels::with_position(&mut channels);
        Self {
            channels,
            default_channels,
            connectivity: InteriorMutable::new(MeshConnectivity::new()),
            gen_config: MeshGenerationConfig::default(),
        }
    }

    pub fn bounding_box(&self) -> (Vec3, Vec3) {
        let mut min = Vec3::new(f32::MAX, f32::MAX, f32::MAX);
        let mut max = Vec3::new(f32::MIN, f32::MIN, f32::MIN);
        for v in self.read_positions().iter() {
            if v.1.x < min.x {
                min.x = v.1.x
            }
            if v.1.y < min.y {
                min.y = v.1.y
            }
            if v.1.z < min.z {
                min.z = v.1.z
            }
            if v.1.x > max.x {
                max.x = v.1.x
            }
            if v.1.y > max.y {
                max.y = v.1.y
            }
            if v.1.z > max.z {
                max.z = v.1.z
            }
        }
        let center = Vec3::new(
            (min.x + max.x) / 2.0,
            (min.y + max.y) / 2.0,
            (min.z + max.z) / 2.0,
        );
        let size = Vec3::new(max.x - min.x, max.y - min.y, max.z - min.z);
        (center, size)
    }

    pub fn read_connectivity(&self) -> BorrowedRef<'_, MeshConnectivity> {
        self.connectivity.borrow()
    }

    /// Generates a lambda suitable for calling the `introspect` method on this
    /// mesh's channels.
    pub fn gen_introspect_fn(&self) -> impl Fn(ChannelKeyType) -> Rc<Vec<slotmap::KeyData>> {
        use slotmap::Key;
        let conn = self.read_connectivity();
        let vs: Rc<Vec<_>> = Rc::new(conn.iter_vertices().map(|(id, _)| id.data()).collect());
        let fs: Rc<Vec<_>> = Rc::new(conn.iter_faces().map(|(id, _)| id.data()).collect());
        let hs: Rc<Vec<_>> = Rc::new(conn.iter_halfedges().map(|(id, _)| id.data()).collect());
        move |k: ChannelKeyType| match k {
            ChannelKeyType::VertexId => vs.clone(),
            ChannelKeyType::FaceId => fs.clone(),
            ChannelKeyType::HalfEdgeId => hs.clone(),
        }
    }

    pub fn write_connectivity(&self) -> MutableRef<'_, MeshConnectivity> {
        self.connectivity.borrow_mut()
    }

    pub fn read_positions(&self) -> BorrowedRef<'_, Positions> {
        self.channels
            .read_channel(self.default_channels.position)
            .expect("Could not read positions")
    }

    pub fn read_face_normals(&self) -> Option<BorrowedRef<'_, Channel<FaceId, Vec3>>> {
        self.default_channels.face_normals.map(|ch_id| {
            self.channels
                .read_channel(ch_id)
                .expect("Could not read face normals")
        })
    }

    pub fn read_vertex_normals(&self) -> Option<BorrowedRef<'_, Channel<VertexId, Vec3>>> {
        self.default_channels.vertex_normals.map(|ch_id| {
            self.channels
                .read_channel(ch_id)
                .expect("Could not read vertex normals")
        })
    }

    pub fn read_uvs(&self) -> Option<BorrowedRef<'_, Channel<HalfEdgeId, Vec3>>> {
        self.default_channels.uvs.map(|ch_id| {
            self.channels
                .read_channel(ch_id)
                .expect("Could not read uvs")
        })
    }

    pub fn write_positions(&self) -> MutableRef<'_, Positions> {
        self.channels
            .write_channel(self.default_channels.position)
            .expect("Could not write positions")
    }

    /// Builds this mesh from a list of vertices, and a list of polygons,
    /// containing indices that reference those vertices.
    ///
    /// - Generic over Index: Use as much precision as you need / want.
    /// - Generic over Polygon: Use whatever input layout you want.
    ///
    /// If unsure, you can pass `Vec<Vec<u32>>` as `polygons`. You can also use
    /// `[[u32;3]]` or `&[&[u32]]`. Same for `u8`, `u16` or `usize` indices.
    pub fn build_from_polygons<Index, Polygon>(
        positions: &[Vec3],
        polygons: &[Polygon],
    ) -> Result<Self>
    where
        Index: num_traits::AsPrimitive<usize> + 'static + Eq + PartialEq + core::hash::Hash + Copy,
        Polygon: AsRef<[Index]>,
    {
        let mesh = Self::new();
        let mut conn = mesh.write_connectivity();
        let mut positions_ch = mesh.write_positions();

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
                let position = positions.get(index.as_()).ok_or_else(|| {
                    anyhow!("Out-of-bounds index in the polygon array {}", index.as_())
                })?;
                let v_id = index_to_vertex
                    .entry(*index)
                    .or_insert_with(|| conn.alloc_vertex(&mut positions_ch, *position, None));

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

            let face = conn.alloc_face(None);

            for (&a, &b) in polygon.iter().circular_tuple_windows() {
                if pair_to_halfedge.get(&(a, b)).is_some() {
                    bail!(
                        "Found multiple oriented edges with the same indices.\
                         This means either (i) surface is non-manifold or (ii) faces \
                         are not oriented in the same direction"
                    )
                }

                let h = conn.alloc_halfedge(HalfEdge::default());
                // Link halfedge to face
                conn[h].face = Some(face);
                conn[face].halfedge = Some(h);

                // Link halfedge to source vertex
                let v_a = index_to_vertex[&a];
                conn[h].vertex = Some(v_a);
                conn[v_a].halfedge = Some(h);

                half_edges_in_face.push(h);

                pair_to_halfedge.insert((a, b), h);

                if let Some(&other) = pair_to_halfedge.get(&(b, a)) {
                    conn[h].twin = Some(other);
                    conn[other].twin = Some(h);
                }
            }

            for (&h1, &h2) in half_edges_in_face.iter().circular_tuple_windows() {
                conn[h1].next = Some(h2);
            }
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
        conn.add_boundary_halfedges();

        // Do some final manifoldness checks
        for (v, vertex) in conn.iter_vertices() {
            if vertex.halfedge.is_none() {
                bail!("There is at least a single vertex that's disconnected from any polygon");
            }

            // Check that the number of halfedges emanating from this vertex
            // equal the number of polygons containing this vertex. If this
            // doesn't check out, it means our vertex is not a polygon "fan",
            // but some other (thus, non-manifold) structure
            let h0 = conn.at_vertex(v).halfedge().end();
            let mut h = h0;
            let mut count = 0;
            loop {
                if !conn.at_halfedge(h).is_boundary().unwrap() {
                    count += 1;
                }
                h = conn.at_halfedge(h).twin().next().end();

                if h == h0 {
                    break;
                }
            }

            if count != vertex_degree[&v] {
                bail!("At least one of the vertices is not a polygon fan, but some other nonmanifold structure instead.")
            }
        }

        drop(conn);
        drop(positions_ch);
        Ok(mesh)
    }

    /// Merges this halfedge mesh with another one. No additional connectivity
    /// data is generated between the two.
    pub fn merge_with(&mut self, mesh_b: &HalfEdgeMesh) {
        let mut vmap = SecondaryMap::<VertexId, VertexId>::new();
        let mut hmap = SecondaryMap::<HalfEdgeId, HalfEdgeId>::new();
        let mut fmap = SecondaryMap::<FaceId, FaceId>::new();

        let mut a_conn = self.write_connectivity();
        let b_conn = mesh_b.read_connectivity();

        // On a first pass, we reserve new vertices, faces and halfedges without
        // setting any of their pointers and store their ids in a mapping.
        for (vertex_id, _vertex) in b_conn.iter_vertices() {
            vmap.insert(vertex_id, a_conn.alloc_vertex_raw(None));
        }
        for (face_id, _) in b_conn.iter_faces() {
            fmap.insert(face_id, a_conn.alloc_face(None));
        }
        for (halfedge_id, _) in b_conn.iter_halfedges() {
            hmap.insert(
                halfedge_id,
                a_conn.alloc_halfedge(HalfEdge {
                    twin: None,
                    next: None,
                    vertex: None,
                    face: None,
                }),
            );
        }

        // The second pass uses the mapping and the original data to set all the
        // inner pointers.
        for (vertex_id, vertex) in b_conn.iter_vertices() {
            if let Some(h) = vertex.halfedge {
                a_conn[vmap[vertex_id]].halfedge = Some(hmap[h])
            }
        }
        for (face_id, face) in b_conn.iter_faces() {
            if let Some(h) = face.halfedge {
                a_conn[fmap[face_id]].halfedge = Some(hmap[h])
            }
        }
        for (halfedge_id, halfedge) in b_conn.iter_halfedges() {
            if let Some(twin) = halfedge.twin {
                a_conn[hmap[halfedge_id]].twin = Some(hmap[twin]);
            }
            if let Some(next) = halfedge.next {
                a_conn[hmap[halfedge_id]].next = Some(hmap[next]);
            }
            if let Some(vertex) = halfedge.vertex {
                a_conn[hmap[halfedge_id]].vertex = Some(vmap[vertex]);
            }
            if let Some(face) = halfedge.face {
                a_conn[hmap[halfedge_id]].face = Some(fmap[face]);
            }
        }
        drop(a_conn);

        // Finally, once the connectivity data is correct, we merge the channels
        // for both meshes.

        /// We need to create two closures in order for the dynamic code inside
        /// the channels to fetch the relevant data:
        ///
        /// - The list of vertex, face or halfedge ids
        /// - Given a vertex, face or halfedge id of the b mesh, its
        ///   corresponding id in the a mesh
        ///
        /// Doing this in a way that we can still invoke the object-safe methods
        /// of a DynChannelGroup requires a copy of the id vectors and wrapping
        /// them in an Rc. The cost of the Rc is negligible, but the copy may
        /// become an issue for very large meshes. On the other hand, the copy
        /// can also help speed iteration up when there are many channels:
        /// since collected vectors are contiguous, unlike the slotmaps,
        /// there will not be holes and thus no required branching.
        use slotmap::Key;
        let raw_vertices: Rc<Vec<_>> =
            Rc::new(b_conn.iter_vertices().map(|(k, _)| k.data()).collect());
        let raw_faces: Rc<Vec<_>> = Rc::new(b_conn.iter_faces().map(|(k, _)| k.data()).collect());
        let raw_halfedges: Rc<Vec<_>> =
            Rc::new(b_conn.iter_halfedges().map(|(k, _)| k.data()).collect());
        let get_ids = move |kty| match kty {
            ChannelKeyType::VertexId => Rc::clone(&raw_vertices),
            ChannelKeyType::FaceId => Rc::clone(&raw_faces),
            ChannelKeyType::HalfEdgeId => Rc::clone(&raw_halfedges),
        };

        let id_map = |kty, k| match kty {
            ChannelKeyType::VertexId => vmap[VertexId::from(k)].data(),
            ChannelKeyType::FaceId => fmap[FaceId::from(k)].data(),
            ChannelKeyType::HalfEdgeId => hmap[HalfEdgeId::from(k)].data(),
        };

        self.channels.merge_with(&mesh_b.channels, get_ids, id_map)
    }
}

impl Default for HalfEdgeMesh {
    fn default() -> Self {
        Self::new()
    }
}

pub trait HalfEdgeOp {
    fn op(conn: &MeshConnectivity, h: HalfEdgeId) -> HalfEdgeId;
}

pub struct NextOp;
impl HalfEdgeOp for NextOp {
    fn op(conn: &MeshConnectivity, h: HalfEdgeId) -> HalfEdgeId {
        conn.at_halfedge(h).next().end()
    }
}

pub struct CycleFanOp;
impl HalfEdgeOp for CycleFanOp {
    fn op(conn: &MeshConnectivity, h: HalfEdgeId) -> HalfEdgeId {
        conn.at_halfedge(h).cycle_around_fan().end()
    }
}

pub struct HalfedgeOpIterator<'a, Op: HalfEdgeOp> {
    conn: &'a MeshConnectivity,
    start: HalfEdgeId,
    next: HalfEdgeId,
    count: usize,
    _op: PhantomData<Op>,
}

impl<'a, Op: HalfEdgeOp> Iterator for HalfedgeOpIterator<'a, Op> {
    type Item = HalfEdgeId;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count >= MAX_LOOP_ITERATIONS {
            panic!("Max number of iterations reached. Is the mesh malformed?");
        } else if self.count > 0 && self.next == self.start {
            None
        } else {
            let res = self.next;
            self.next = Op::op(self.conn, self.next);
            self.count += 1;
            Some(res)
        }
    }
}

impl Vertex {
    pub fn introspect(&self, h_mapping: &MeshMapping<HalfEdgeId>) -> String {
        let h = self.halfedge.map(|h| h_mapping[h]);
        format!("halfedge: {h:?}")
    }
}

impl Face {
    pub fn introspect(&self, h_mapping: &MeshMapping<HalfEdgeId>) -> String {
        let h = self.halfedge.map(|h| h_mapping[h]);
        format!("halfedge: {h:?}")
    }
}

impl HalfEdge {
    pub fn introspect(
        &self,
        h_mapping: &MeshMapping<HalfEdgeId>,
        v_mapping: &MeshMapping<VertexId>,
        f_mapping: &MeshMapping<FaceId>,
    ) -> String {
        let next = self.next.map(|h| h_mapping[h]);
        let twin = self.twin.map(|h| h_mapping[h]);
        let face = self.face.map(|f| f_mapping[f]);
        let vertex = self.vertex.map(|v| v_mapping[v]);
        format!("next: {next:?}\ntwin: {twin:?}\nface: {face:?}\nvertex: {vertex:?}")
    }
}

pub mod mappings;

impl MeshConnectivity {
    pub fn vertex_mapping(&self) -> mappings::MeshMapping<VertexId> {
        mappings::MeshMapping::new(&self.vertices)
    }

    pub fn face_mapping(&self) -> mappings::MeshMapping<FaceId> {
        mappings::MeshMapping::new(&self.faces)
    }

    pub fn halfedge_mapping(&self) -> mappings::MeshMapping<HalfEdgeId> {
        mappings::MeshMapping::new(&self.halfedges)
    }
}
