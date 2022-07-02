use std::f32::consts::PI;

use super::*;

pub struct Box;

impl Box {
    pub fn build(center: Vec3, size: Vec3) -> HalfEdgeMesh {
        let hsize = size * 0.5;

        let v1 = center + Vec3::new(-hsize.x, -hsize.y, -hsize.z);
        let v2 = center + Vec3::new(hsize.x, -hsize.y, -hsize.z);
        let v3 = center + Vec3::new(hsize.x, -hsize.y, hsize.z);
        let v4 = center + Vec3::new(-hsize.x, -hsize.y, hsize.z);

        let v5 = center + Vec3::new(-hsize.x, hsize.y, -hsize.z);
        let v6 = center + Vec3::new(-hsize.x, hsize.y, hsize.z);
        let v7 = center + Vec3::new(hsize.x, hsize.y, hsize.z);
        let v8 = center + Vec3::new(hsize.x, hsize.y, -hsize.z);

        /*
               // Top
               hem.add_quad(v1, v2, v3, v4);
               //Bottom
               hem.add_quad(v5, v6, v7, v8);
               // Front
               hem.add_quad(v5, v8, v2, v1);
               // Back
               hem.add_quad(v4, v3, v7, v6);
               // Left
               hem.add_quad(v6, v5, v1, v4);
               // Right
               hem.add_quad(v7, v3, v2, v8);
        */
        HalfEdgeMesh::build_from_polygons(
            &[v1, v2, v3, v4, v5, v6, v7, v8],
            &[
                &[0, 1, 2, 3],
                &[4, 5, 6, 7],
                &[4, 7, 1, 0],
                &[3, 2, 6, 5],
                &[5, 4, 0, 3],
                &[6, 2, 1, 7],
            ],
        )
        .expect("Cube construction should not fail")
    }
}

pub struct Quad;
impl Quad {
    pub fn build(center: Vec3, normal: Vec3, right: Vec3, size: Vec2) -> HalfEdgeMesh {
        let normal = normal.normalize();
        let right = right.normalize();
        let forward = normal.cross(right);

        let hsize = size * 0.5;

        let v1 = center + hsize.x * right + hsize.y * forward;
        let v2 = center - hsize.x * right + hsize.y * forward;
        let v3 = center - hsize.x * right - hsize.y * forward;
        let v4 = center + hsize.x * right - hsize.y * forward;

        HalfEdgeMesh::build_from_polygons(&[v1, v2, v3, v4], &[&[0, 1, 2, 3]])
            .expect("Quad construction should not fail")
    }
}

pub struct Circle;
impl Circle {
    pub fn build(center: Vec3, radius: f32, num_vertices: usize) -> HalfEdgeMesh {
        let angle_delta = (2.0 * PI) / num_vertices as f32;
        let verts = (0..num_vertices)
            .map(|i| {
                let q = Quat::from_rotation_y(angle_delta * i as f32);
                q * (Vec3::Z * radius) + center
            })
            .collect_vec();
        let polygon = (0..num_vertices).collect_vec();

        HalfEdgeMesh::build_from_polygons(&verts, &[&polygon])
            .expect("Circle construction should not fail")
    }

    pub fn build_open(center: Vec3, radius: f32, num_vertices: usize) -> HalfEdgeMesh {
        let circle = Self::build(center, radius, num_vertices);
        {
            let mut conn = circle.write_connectivity();
            let (v, _) = conn.iter_vertices().next().unwrap();
            let halfedge = conn.at_vertex(v).halfedge().end();
            let face = conn.at_halfedge(halfedge).face().end();

            // Clear the face
            for h in conn.halfedge_loop(halfedge) {
                conn[h].face = None;
            }
            conn.remove_face(face);
        }
        circle
    }
}

pub struct UVSphere;
impl UVSphere {
    pub fn build(center: Vec3, segments: u32, rings: u32, radius: f32) -> HalfEdgeMesh {
        let mut vertices = Vec::<Vec3>::new();
        let mut polygons = Vec::<SVec<u32>>::new();

        let top_vertex = 0;
        vertices.push(center + Vec3::Y * radius);

        for i in 0..rings - 1 {
            let phi = PI * (i + 1) as f32 / rings as f32;
            for j in 0..segments {
                let theta = 2.0 * PI * j as f32 / segments as f32;
                let x = phi.sin() * theta.cos() * radius;
                let y = phi.cos() * radius;
                let z = phi.sin() * theta.sin() * radius;
                vertices.push(center + Vec3::new(x, y, z));
            }
        }

        let bottom_vertex = vertices.len() as u32;
        vertices.push(center - Vec3::Y * radius);

        // Top triangles
        for i in 0..segments {
            let i0 = i + 1;
            let i1 = (i + 1) % segments + 1;
            polygons.push(smallvec::smallvec![top_vertex, i1, i0]);
        }
        // Bottom triangles
        for i in 0..segments {
            let i0 = i + segments * (rings - 2) + 1;
            let i1 = (i + 1) % segments + segments * (rings - 2) + 1;
            polygons.push(smallvec::smallvec![bottom_vertex, i0, i1]);
        }
        // Middle quads
        for j in 0..rings - 2 {
            let j0 = j * segments + 1;
            let j1 = (j + 1) * segments + 1;
            for i in 0..segments {
                let i0 = j0 + i;
                let i1 = j0 + (i + 1) % segments;
                let i2 = j1 + (i + 1) % segments;
                let i3 = j1 + i;
                polygons.push(smallvec::smallvec![i0, i1, i2, i3]);
            }
        }

        HalfEdgeMesh::build_from_polygons(&vertices, &polygons)
            .expect("Sphere construction should not fail")
    }
}

pub struct Line;
impl Line {
    pub fn build(start: Vec3, end: Vec3, segments: u32) -> HalfEdgeMesh {
        let mesh = HalfEdgeMesh::new();
        let mut conn = mesh.write_connectivity();
        let mut pos = mesh.write_positions();

        let mut forward_halfedges = SVec::new();
        let mut backward_halfedges = SVec::new();

        let mut v = conn.alloc_vertex(&mut pos, start, None);
        for i in 0..segments {
            let w = conn.alloc_vertex(
                &mut pos,
                start.lerp(end, (i+1) as f32 / segments as f32),
                None,
            );

            let h_v_w = conn.alloc_halfedge(HalfEdge {
                twin: None,
                next: None,
                vertex: Some(v),
                face: None,
            });
            let h_w_v = conn.alloc_halfedge(HalfEdge {
                twin: None,
                next: None,
                vertex: Some(w),
                face: None,
            });

            conn[h_v_w].twin = Some(h_w_v);
            conn[h_w_v].twin = Some(h_v_w);

            conn[v].halfedge = Some(h_v_w);
            conn[w].halfedge = Some(h_w_v);

            forward_halfedges.push(h_v_w);
            backward_halfedges.push(h_w_v);

            // For the next iteration, repeat same operation starting at w
            v = w;
        }

        // Make a chain with all the halfedges in the line
        for (h, h2) in forward_halfedges.iter_cpy().tuple_windows() {
            conn[h].next = Some(h2);
        }
        for (h, h2) in backward_halfedges.iter_cpy().rev().tuple_windows() {
            conn[h].next = Some(h2);
        }
        
        // Tie the ends together, forming a loop
        let f_h_first = forward_halfedges.iter_cpy().next().expect("At least one halfedge");
        let f_h_last = forward_halfedges.iter_cpy().last().expect("At least one halfedge");
        let b_h_first = backward_halfedges.iter_cpy().next().expect("At least one halfedge");
        let b_h_last = backward_halfedges.iter_cpy().last().expect("At least one halfedge");
        conn[f_h_last].next = Some(b_h_last);
        conn[b_h_first].next = Some(f_h_first);

        drop(conn);
        drop(pos);
        
        mesh
    }
}
