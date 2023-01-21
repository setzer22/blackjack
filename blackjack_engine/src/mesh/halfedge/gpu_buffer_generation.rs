// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

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
    /// Face colors, N for N triangles. Includes alpha channel.
    pub colors: Vec<Vec4>,
    /// Face ids, N for N triangles
    pub ids: Vec<u32>,
    /// The largest id in `ids`.
    pub max_id: u32,
}

impl HalfEdgeMesh {
    /// Generates the [`TriangleBuffers`] for this mesh. Suitable to be uploaded
    /// to the GPU.
    #[profiling::function]
    pub fn generate_triangle_buffers_flat(&self, force_gen: bool) -> Result<VertexIndexBuffers> {
        let positions_ch = self.read_positions();
        let conn = self.read_connectivity();

        // This ugliness is needed because we need to either borrow the channel
        // for the mesh or generate it here, but if we generate inside the if
        // statement the ref owner gets dropped at the end of the scope.
        let normal_ch: &Channel<_, _>;
        #[allow(unused_assignments)] // Look ma, I'm smarter than clippy
        let mut extend_lifetime = None;
        let existing_normals_ch = self.read_face_normals();
        if !force_gen && existing_normals_ch.is_some() {
            normal_ch = existing_normals_ch.as_deref().unwrap();
        } else {
            extend_lifetime = Some(edit_ops::generate_flat_normals_channel(self)?);
            normal_ch = extend_lifetime.as_ref().unwrap();
        }

        let mut positions = vec![];
        let mut normals = vec![];

        for (face_id, _face) in conn.faces.iter() {
            // We try to be a bit forgiving here. We don't want to stop
            // rendering even if we have slightly malformed meshes.
            let normal = normal_ch[face_id];

            let vertices = conn.face_vertices(face_id);

            let v1 = vertices[0];

            for (&v2, &v3) in vertices[1..].iter().tuple_windows() {
                positions.push(positions_ch[v1]);
                positions.push(positions_ch[v2]);
                positions.push(positions_ch[v3]);
                normals.push(normal);
                normals.push(normal);
                normals.push(normal);
            }
        }

        Ok(VertexIndexBuffers {
            indices: (0u32..positions.len() as u32).collect(),
            positions,
            normals,
        })
    }

    /// If `force_gen` is true, ignores any existing vertex normals channel in
    /// the mesh and generates one from scratch instead. This is used in some
    /// viewport modes.
    pub fn generate_triangle_buffers_smooth(&self, force_gen: bool) -> Result<VertexIndexBuffers> {
        let positions_ch = self.read_positions();
        let conn = self.read_connectivity();

        // @CopyPaste -- This couldn't get any worse... Are we Java now or what?
        // This ugliness is needed because we need to either borrow the channel
        // for the mesh or generate it here, but if we generate inside the if
        // statement the ref owner gets dropped at the end of the scope.
        let normal_ch: &Channel<_, _>;
        #[allow(unused_assignments)] // Look ma, I'm smarter than clippy
        let mut extend_lifetime = None;
        let existing_normals_ch = self.read_vertex_normals();
        if !force_gen && existing_normals_ch.is_some() {
            normal_ch = existing_normals_ch.as_deref().unwrap();
        } else {
            extend_lifetime = Some(edit_ops::generate_smooth_normals_channel(self)?);
            normal_ch = extend_lifetime.as_ref().unwrap();
        }

        let mut v_id_to_idx =
            slotmap::SecondaryMap::<VertexId, u32>::with_capacity(conn.vertices.capacity());
        let mut positions = vec![];
        let mut normals = vec![];

        conn.iter_vertices_with_channel(&positions_ch)
            .enumerate()
            .try_for_each::<_, Result<()>>(|(idx, (v_id, _v, pos))| {
                v_id_to_idx.insert(v_id, idx as u32);
                positions.push(pos);
                normals.push(normal_ch[v_id]);
                Ok(())
            })?;

        let mut indices = vec![];
        for (face_id, _face) in conn.faces.iter() {
            let vertices = conn.face_vertices(face_id);
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

    pub fn generate_face_overlay_buffers(&self, hover: Option<u32>) -> FaceOverlayBuffers {
        let positions_ch = self.read_positions();
        let conn = self.read_connectivity();

        let mut positions = vec![];
        let mut colors = vec![];
        let mut ids = vec![];
        let mut max_id = 0;

        // TODO @perf We could reuse this mapping when we extract the
        // information from the compute shader to map back to the hovered ids,
        // but for now let's keep it simple and recompute this when needed.
        let mapping = conn.face_mapping();

        for (_, (face_id, _face)) in conn.faces.iter().enumerate() {
            let id_u32 = mapping[face_id];
            max_id = u32::max(max_id, id_u32);

            let vertices = conn.face_vertices(face_id);
            let v1 = vertices[0];
            for (&v2, &v3) in vertices[1..].iter().tuple_windows() {
                let v1_pos = positions_ch[v1];
                let v2_pos = positions_ch[v2];
                let v3_pos = positions_ch[v3];

                // NOTE: We add 1 to the ids because 0 is the clear color of the
                // id buffer, so we need a way to distinguish actual ids, and
                // zero is an otherwise valid id.
                let id = id_u32 + 1;
                let color_alpha = if hover.is_some_and_(|h| *h == id) {
                    0.5
                } else {
                    0.0
                };

                positions.push(v1_pos);
                positions.push(v2_pos);
                positions.push(v3_pos);
                colors.push(Vec4::new(0.2, 0.8, 0.2, color_alpha));
                ids.push(id_u32 + 1);
            }
        }

        FaceOverlayBuffers {
            positions,
            colors,
            ids,
            max_id,
        }
    }

    /// Generates the [`PointBuffers`] for this mesh. Suitable to be uploaded to
    /// the GPU.
    pub fn generate_point_buffers(&self) -> PointBuffers {
        let mut positions = Vec::new();
        for (_, _, pos) in self
            .read_connectivity()
            .iter_vertices_with_channel(&self.read_positions())
        {
            positions.push(pos)
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
        let positions_ch = self.read_positions();
        let conn = self.read_connectivity();

        let mut visited = HashSet::new();
        let mut positions = Vec::new();
        let mut colors = Vec::new();

        for (h, halfedge) in conn.iter_halfedges() {
            let tw = halfedge
                .twin
                .ok_or_else(|| anyhow!("All halfedges should have a twin"))?;
            if visited.contains(&tw) {
                continue;
            } else {
                visited.insert(h);
            }

            let (src, dst) = conn.at_halfedge(h).src_dst_pair().map_err(|err| {
                anyhow!("All halfedges should have src and dst vertices: {}", err)
            })?;

            positions.push(positions_ch[src]);
            positions.push(positions_ch[dst]);

            if let Some(dbg_edge) = conn.debug_edges.get(&h) {
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
        let positions_ch = self.read_positions();
        let conn = self.read_connectivity();

        let mut colors = vec![];
        let mut positions = vec![];

        for (h, _) in conn.iter_halfedges() {
            let (src, dst) = conn.at_halfedge(h).src_dst_pair()?;

            let src_pos = positions_ch[src];
            let dst_pos = positions_ch[dst];
            let edge_length = (dst_pos - src_pos).length();

            let separation = edge_length * 0.1;
            let shrink = edge_length * 0.2;

            let midpoint = (src_pos + dst_pos) * 0.5;
            let face_centroid = conn
                .at_halfedge(h)
                .face()
                .try_end()
                .map(|face| conn.face_vertex_average(&positions_ch, face));
            let towards_face = if let Ok(centroid) = face_centroid {
                (centroid - midpoint).normalize() * separation
            } else {
                Vec3::ZERO
            };

            let bitangent = (dst_pos - src_pos).normalize();

            let src_pos = src_pos + towards_face + bitangent * shrink;
            let dst_pos = dst_pos + towards_face - bitangent * shrink;

            let normal = if let Some(face) = conn.at_halfedge(h).face_or_boundary()? {
                conn.face_normal(&positions_ch, face).unwrap_or(Vec3::ZERO)
            } else if let Some(twin_face) = conn.at_halfedge(h).twin().face_or_boundary()? {
                conn.face_normal(&positions_ch, twin_face)
                    .unwrap_or(Vec3::ZERO)
            } else {
                Vec3::Y
            };

            let tangent = normal.cross(bitangent);

            positions.extend([src_pos, dst_pos]);
            positions.extend([
                dst_pos,
                dst_pos + 0.30 * edge_length * tangent.lerp(-bitangent, 2.0 / 3.0),
            ]);

            if let Some(dbg_edge) = conn.debug_edges.get(&h) {
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
