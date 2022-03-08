use super::*;

/// The main representation to draw the halfedge's faces as triangles on the GPU
/// This is suitable to be rendered with `wgpu::PrimitiveTopology::TriangleList`
#[derive(Clone, Debug)]
pub struct VertexIndexBuffers {
    /// Vertex positions, one per vertex.
    pub positions: Vec<Vec3>,
    /// Vertex normals, one per vertex.
    pub normals: Vec<Vec3>,
    /// Indices: 3*N where N is the number of triangles. Indices point to
    /// elements of `positions` and `normals`.
    pub indices: Vec<u32>,
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

/// This representation is used to draw highlighted flat triangles over a base
/// mesh. It is used to draw a selection of faces.
pub struct FaceOverlayBuffers {
    /// Vertex positions, 3*N, for N triangles
    pub positions: Vec<Vec3>,
    /// Face colors, N for N triangles
    pub colors: Vec<Vec3>,
}

impl HalfEdgeMesh {
    /// Generates the [`TriangleBuffers`] for this mesh. Suitable to be uploaded
    /// to the GPU.
    #[profiling::function]
    pub fn generate_triangle_buffers_flat(&self) -> VertexIndexBuffers {
        let mut positions = vec![];
        let mut normals = vec![];

        for (face_id, _face) in self.faces.iter() {
            // We try to be a bit forgiving here. We don't want to stop
            // rendering even if we have slightly malformed meshes.
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
                normals.push(normal);
                normals.push(normal);
                normals.push(normal);
            }
        }

        VertexIndexBuffers {
            indices: (0u32..positions.len() as u32).collect(),
            positions,
            normals,
        }
    }

    pub fn generate_triangle_buffers_smooth(&self) -> Result<VertexIndexBuffers> {
        let mut v_id_to_idx =
            slotmap::SecondaryMap::<VertexId, u32>::with_capacity(self.vertices.capacity());
        let mut positions = vec![];
        let mut normals = vec![];

        self.iter_vertices()
            .enumerate()
            .try_for_each::<_, Result<()>>(|(idx, (id, v))| {
                v_id_to_idx.insert(id, idx as u32);
                positions.push(v.position);

                let adjacent_faces = self.at_vertex(id).adjacent_faces()?;
                let mut normal = Vec3::ZERO;
                for face in adjacent_faces.iter_cpy() {
                    normal += self.face_normal(face).unwrap_or(Vec3::ZERO);
                }
                normals.push(normal / adjacent_faces.len() as f32);
                Ok(())
            })?;

        let mut indices = vec![];
        for (face_id, _face) in self.faces.iter() {
            let vertices = self.face_vertices(face_id);
            let v1 = vertices[0];
            for (&v2, &v3) in vertices[1..].iter().tuple_windows() {
                indices.push(v_id_to_idx[v1]);
                indices.push(v_id_to_idx[v2]);
                indices.push(v_id_to_idx[v3]);
            }
        }

        Ok(VertexIndexBuffers {
            positions,
            normals,
            indices,
        })
    }

    pub fn generate_face_overlay_buffers(&self) -> FaceOverlayBuffers {
        let mut positions = vec![];
        let mut colors = vec![];

        for (_, (face_id, _face)) in self.faces.iter().enumerate() {
            // TODO: Add criteria to select highlighted faces
            if false {
                let vertices = self.face_vertices(face_id);
                let v1 = vertices[0];
                for (&v2, &v3) in vertices[1..].iter().tuple_windows() {
                    let v1_pos = self[v1].position;
                    let v2_pos = self[v2].position;
                    let v3_pos = self[v3].position;

                    positions.push(v1_pos);
                    positions.push(v2_pos);
                    positions.push(v3_pos);
                    colors.push(Vec3::new(0.2, 0.8, 0.2));
                }
            }
        }

        FaceOverlayBuffers { positions, colors }
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
    pub fn generate_line_buffers(&self) -> Result<LineBuffers> {
        let mut visited = HashSet::new();
        let mut positions = Vec::new();
        let mut colors = Vec::new();
        for (h, halfedge) in self.iter_halfedges() {
            let tw = halfedge
                .twin
                .ok_or(anyhow!("All halfedges should have a twin"))?;
            if visited.contains(&tw) {
                continue;
            } else {
                visited.insert(h);
            }

            let (src, dst) = self.at_halfedge(h).src_dst_pair().map_err(|err| {
                anyhow!("All halfedges should have src and dst vertices: {}", err)
            })?;

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

        Ok(LineBuffers { colors, positions })
    }

    /// Generates a variation of the [`LineBuffers`] which can be drawn in the
    /// exact same way, but instead of drawing a single line per edge, draws
    /// halfedges individually as tiny arrows.
    pub fn generate_halfedge_arrow_buffers(&self) -> Result<LineBuffers> {
        let mut colors = vec![];
        let mut positions = vec![];

        for (h, _) in self.iter_halfedges() {
            let (src, dst) = self.at_halfedge(h).src_dst_pair()?;

            let src_pos = self.vertex_position(src);
            let dst_pos = self.vertex_position(dst);
            let edge_length = (dst_pos - src_pos).length();

            let separation = edge_length * 0.1;
            let shrink = edge_length * 0.2;

            let midpoint = (src_pos + dst_pos) * 0.5;
            let face_centroid = self
                .at_halfedge(h)
                .face()
                .try_end()
                .map(|face| self.face_vertex_average(face));
            let towards_face = if let Ok(centroid) = face_centroid {
                (centroid - midpoint).normalize() * separation
            } else {
                Vec3::ZERO
            };

            let bitangent = (dst_pos - src_pos).normalize();

            let src_pos = src_pos + towards_face + bitangent * shrink;
            let dst_pos = dst_pos + towards_face - bitangent * shrink;

            let normal = if let Some(face) = self.at_halfedge(h).face_or_boundary()? {
                self.face_normal(face).unwrap_or(Vec3::ZERO)
            } else if let Some(twin_face) = self.at_halfedge(h).twin().face_or_boundary()? {
                self.face_normal(twin_face).unwrap_or(Vec3::ZERO)
            } else {
                Vec3::ZERO
            };

            let tangent = normal.cross(bitangent);

            positions.extend(&[src_pos, dst_pos]);

            positions.extend(&[
                dst_pos,
                dst_pos + 0.30 * edge_length * tangent.lerp(-bitangent, 2.0 / 3.0),
            ]);

            if let Some(dbg_edge) = self.debug_edges.get(&h) {
                let color = glam::Vec3::new(
                    dbg_edge.color.r() as f32 / 255.0,
                    dbg_edge.color.g() as f32 / 255.0,
                    dbg_edge.color.b() as f32 / 255.0,
                );
                colors.push(color);
                colors.push(color);
            } else {
                colors.push(Vec3::splat(1.0));
                colors.push(Vec3::splat(1.0));
            }
        }

        Ok(LineBuffers { colors, positions })
    }
}
