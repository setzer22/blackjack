use super::*;

/// The main representation to draw the halfedge's faces as triangles on the GPU
/// This is suitable to be rendered with `wgpu::PrimitiveTopology::TriangleList`
#[derive(Clone, Debug)]
pub struct TriangleBuffers {
    /// Vertex positions, 3*N for N triangular faces
    pub positions: Vec<Vec3>,
    /// Face normals, N, one per face
    pub normals: Vec<Vec3>,
    /// Vertex colors, N, one per face.
    pub colors: Vec<Vec3>,
}

/// This representation is suitable to draw the halfedge's vertices using
/// `wgpu::PrimitiveTopology::PointList`.
///
/// Note that this structure has no indices because that would be pointless
/// Indices can be generated as the sequence 1..N where N is the length of the
/// `positions` buffer
pub struct PointBuffers {
    /// Vertex positions
    pub positions: Vec<Vec3>,
}

/// This representation is suitable to draw the halfedge's vertices using
/// `wgpu::PrimitiveTopology::LineList`.
pub struct LineBuffers {
    pub positions: Vec<Vec3>,
    pub colors: Vec<Vec3>,
}

impl HalfEdgeMesh {
    /// Generates the [`TriangleBuffers`] for this mesh. Suitable to be uploaded
    /// to the GPU.
    #[profiling::function]
    pub fn generate_triangle_buffers(&self) -> TriangleBuffers {
        let mut done_faces: HashSet<FaceId> = HashSet::new();

        let mut positions = vec![];
        let mut normals = vec![];
        let mut colors = vec![];

        for (face_id, _face) in self.faces.iter() {
            // TODO: I think this is a leftover from an old refactor. It makes
            // no sense to check for duplicate faces when iterating faces. Need
            // to check if this is useful, remove otherwise.
            if done_faces.contains(&face_id) {
                continue;
            }
            done_faces.insert(face_id);

            let normal = self.face_normal(face_id).unwrap_or(Vec3::ZERO);

            let vertices = self.face_vertices(face_id);

            let v1 = vertices[0];

            for (&v2, &v3) in vertices[1..].iter().tuple_windows() {
                let v1_pos = self[v1].position;
                let v2_pos = self[v2].position;
                let v3_pos = self[v3].position;

                positions.push(v1_pos);
                positions.push(v2_pos);
                positions.push(v3_pos);
                colors.push(Vec3::splat(1.0)); // TODO per-face colors
                normals.push(normal);
            }
        }

        TriangleBuffers { positions, colors, normals }
    }

    /// Generates the [`PointBuffers`] for this mesh. Suitable to be uploaded to
    /// the GPU.
    pub fn generate_point_buffers(&self) -> PointBuffers {
        let mut positions = Vec::new();
        for (_, vertex) in self.iter_vertices() {
            positions.push(vertex.position)
        }
        PointBuffers { positions }
    }

    /// Generates the [`LineBuffers`] for this mesh. Suitable to be uploaded to
    /// the GPU.
    ///
    /// # Panics
    /// This method panics if the mesh is malformed:
    /// - When a halfedge does not have a twin
    /// - When a halfedge does not have (src, dst) vertices
    pub fn generate_line_buffers(&self) -> LineBuffers {
        let mut visited = HashSet::new();
        let mut positions = Vec::new();
        let mut colors = Vec::new();
        for (h, halfedge) in self.iter_halfedges() {
            let tw = halfedge.twin.expect("All halfedges should have a twin");
            if visited.contains(&tw) {
                continue;
            } else {
                visited.insert(h);
            }

            let (src, dst) = self
                .at_halfedge(h)
                .src_dst_pair()
                .expect("All halfedges should have src and dst vertices");

            positions.push(self.vertex_position(src));
            positions.push(self.vertex_position(dst));

            if let Some(dbg_edge) = self.debug_edges.get(&h) {
                let color = glam::Vec3::new(
                    dbg_edge.color.r() as f32 / 255.0,
                    dbg_edge.color.g() as f32 / 255.0,
                    dbg_edge.color.b() as f32 / 255.0,
                );
                colors.push(color)
            } else {
                colors.push(Vec3::splat(1.0))
            }
        }

        LineBuffers { colors, positions }
    }
}
