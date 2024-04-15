// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::f32::consts::PI;

use super::*;

pub struct Box;

impl Box {
    pub fn build(center: Vec3, size: Vec3) -> Result<HalfEdgeMesh> {
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
    }
}

pub struct Quad;
impl Quad {
    pub fn build(center: Vec3, normal: Vec3, right: Vec3, size: Vec2) -> Result<HalfEdgeMesh> {
        let normal = normal.normalize();
        let right = right.normalize();
        let forward = normal.cross(right);

        let hsize = size * 0.5;

        let v1 = center + hsize.x * right + hsize.y * forward;
        let v2 = center - hsize.x * right + hsize.y * forward;
        let v3 = center - hsize.x * right - hsize.y * forward;
        let v4 = center + hsize.x * right - hsize.y * forward;

        HalfEdgeMesh::build_from_polygons(&[v1, v2, v3, v4], &[&[0, 1, 2, 3]])
    }
}

pub struct Circle;
impl Circle {
    pub fn make_verts(center: Vec3, radius: f32, num_vertices: usize) -> Vec<Vec3> {
        let angle_delta = (2.0 * PI) / num_vertices as f32;
        (0..num_vertices)
            .map(|i| {
                let q = Quat::from_rotation_y(angle_delta * i as f32);
                q * (Vec3::Z * radius) + center
            })
            .collect_vec()
    }

    pub fn build(center: Vec3, radius: f32, num_vertices: usize) -> Result<HalfEdgeMesh> {
        let verts = Self::make_verts(center, radius, num_vertices);
        let polygon = (0..num_vertices).collect_vec();

        HalfEdgeMesh::build_from_polygons(&verts, &[&polygon])
    }

    pub fn build_open(center: Vec3, radius: f32, num_vertices: usize) -> Result<HalfEdgeMesh> {
        let circle = Self::build(center, radius, num_vertices)?;
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
        Ok(circle)
    }
}

pub struct UVSphere;
impl UVSphere {
    pub fn build(center: Vec3, segments: u32, rings: u32, radius: f32) -> Result<HalfEdgeMesh> {
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
    }
}

pub struct Line;
impl Line {
    pub fn build(position: &impl Fn(u32) -> Vec3, segments: u32) -> Result<HalfEdgeMesh> {
        let tangent = |i| {
            match i {
                0 => position(1) - position(0),
                i if i == segments => position(segments) - position(segments - 1),
                i => {
                    let n1 = (position(i) - position(i - 1)).normalize();
                    let n2 = (position(i + 1) - position(i)).normalize();
                    n1 + n2
                }
            }
            .normalize()
        };
        let normal = |i| tangent(i).any_orthonormal_vector();
        Self::build_with_normals(&position, &normal, &tangent, segments)
    }

    pub fn build_with_normals(
        position: &impl Fn(u32) -> Vec3,
        normal: &impl Fn(u32) -> Vec3,
        tangent: &impl Fn(u32) -> Vec3,
        segments: u32,
    ) -> Result<HalfEdgeMesh> {
        if segments == 0 {
            return HalfEdgeMesh::build_from_polygons::<usize, &[usize]>(&[position(0)], &[]);
        }
        let mut mesh = HalfEdgeMesh::new();
        let tangent_channel_id = mesh.channels.ensure_channel::<VertexId, Vec3>("tangent");
        let normal_channel_id = mesh.channels.ensure_channel::<VertexId, Vec3>("normal");
        let mut conn = mesh.write_connectivity();
        let mut pos = mesh.write_positions();
        let mut norm = mesh.channels.write_channel(normal_channel_id)?;
        let mut tang = mesh.channels.write_channel(tangent_channel_id)?;

        let mut forward_halfedges = SVec::new();
        let mut backward_halfedges = SVec::new();

        let mut v = conn.alloc_vertex(&mut pos, position(0), None);
        norm[v] = normal(0);
        tang[v] = tangent(0);
        for i in 1..=segments {
            let w = conn.alloc_vertex(&mut pos, position(i), None);
            norm[w] = normal(i);
            tang[w] = tangent(i);

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
        let f_h_first = forward_halfedges
            .iter_cpy()
            .next()
            .context("expected at least one halfedge")?;
        let f_h_last = forward_halfedges
            .iter_cpy()
            .last()
            .context("expected at least one halfedge")?;
        let b_h_first = backward_halfedges
            .iter_cpy()
            .next()
            .context("expected at least one halfedge")?;
        let b_h_last = backward_halfedges
            .iter_cpy()
            .last()
            .context("expected at least one halfedge")?;
        conn[f_h_last].next = Some(b_h_last);
        conn[b_h_first].next = Some(f_h_first);

        drop(conn);
        drop(pos);
        drop(norm);
        drop(tang);

        Ok(mesh)
    }

    pub fn build_straight_line(start: Vec3, end: Vec3, segments: u32) -> Result<HalfEdgeMesh> {
        Self::build(&|i| start.lerp(end, i as f32 / segments as f32), segments)
    }

    pub fn build_from_points(points: Vec<Vec3>) -> Result<HalfEdgeMesh> {
        match points.len() {
            0 => Ok(HalfEdgeMesh::new()),
            len => Self::build(&|i| points[i as usize], len as u32 - 1),
        }
    }
}

pub struct Polygon;
impl Polygon {
    pub fn build_from_points(points: Vec<Vec3>) -> Result<HalfEdgeMesh> {
        let indices = points
            .iter()
            .enumerate()
            .map(|(i, _)| i as u32)
            .collect_vec();
        HalfEdgeMesh::build_from_polygons(&points, &[&indices])
    }
}

pub struct Cone;
impl Cone {
    pub fn build(
        center: Vec3,
        top_radius: f32,
        bottom_radius: f32,
        height: f32,
        num_vertices: usize,
    ) -> Result<HalfEdgeMesh> {
        if top_radius.abs() <= 1e-5 {
            Self::build_cone(center, bottom_radius, height, num_vertices)
        } else {
            Self::build_truncated_cone(center, top_radius, bottom_radius, height, num_vertices)
        }
    }
    pub fn build_cone(
        center: Vec3,
        bottom_radius: f32,
        height: f32,
        num_vertices: usize,
    ) -> Result<HalfEdgeMesh> {
        let v_offset = Vec3::new(0.0, height / 2.0, 0.0);
        let mut verts = Circle::make_verts(center - v_offset, bottom_radius, num_vertices);
        verts.push(center + v_offset);

        let side_faces = (0..num_vertices)
            .map(|v| [v, (v + 1) % num_vertices, num_vertices])
            .collect_vec();
        let bottom_face = (0..num_vertices).rev().collect_vec();
        let mut faces = vec![bottom_face.as_slice()];
        faces.extend(side_faces.iter().map(|x| x.as_slice()));

        HalfEdgeMesh::build_from_polygons(&verts, &faces)
    }
    pub fn build_truncated_cone(
        center: Vec3,
        top_radius: f32,
        bottom_radius: f32,
        height: f32,
        num_vertices: usize,
    ) -> Result<HalfEdgeMesh> {
        let v_offset = Vec3::new(0.0, height / 2.0, 0.0);
        let mut verts = Circle::make_verts(center - v_offset, bottom_radius, num_vertices);
        verts.extend(Circle::make_verts(
            center + v_offset,
            top_radius,
            num_vertices,
        ));

        let side_faces = (0..num_vertices)
            .map(|v| {
                let v2 = (v + 1) % num_vertices;
                [v, v2, num_vertices + v2, num_vertices + v]
            })
            .collect_vec();
        let bottom_face = (0..num_vertices).rev().collect_vec();
        let top_face = (num_vertices..(2 * num_vertices)).collect_vec();
        let mut faces = vec![bottom_face.as_slice(), top_face.as_slice()];
        faces.extend(side_faces.iter().map(|x| x.as_slice()));

        HalfEdgeMesh::build_from_polygons(&verts, &faces)
    }
}

struct Cylinder;
impl Cylinder {
    pub fn build(
        center: Vec3,
        radius: f32,
        height: f32,
        num_vertices: usize,
    ) -> Result<HalfEdgeMesh> {
        Cone::build_truncated_cone(center, radius, radius, height, num_vertices)
    }
}

pub struct Grid;
impl Grid {
    pub fn build(x: u32, y: u32, spacing_x: f32, spacing_y: f32) -> Result<HalfEdgeMesh> {
        let mesh = HalfEdgeMesh::new();
        let mut conn = mesh.write_connectivity();
        let mut pos = mesh.write_positions();

        for i in 0..x {
            for j in 0..y {
                conn.alloc_vertex(
                    &mut pos,
                    Vec3::new(i as f32 * spacing_x, j as f32 * spacing_y, 0.0),
                    None,
                );
            }
        }

        drop(conn);
        drop(pos);

        Ok(mesh)
    }
}

fn catenary(x: f32, a: f32) -> f32 {
    a * (x / a).cosh()
}

fn catenary_dx(x: f32, a: f32) -> f32 {
    (x / a).sinh()
}

/// Curve of a hanging chain, rope, or wire. https://en.wikipedia.org/wiki/Catenary
pub struct Catenary;
impl Catenary {
    const MAX_ERR: f32 = 1e-2;
    const NEWTON_ITERS: u32 = 20;

    pub fn build(start: Vec3, end: Vec3, sag: f32, segments: u32) -> Result<HalfEdgeMesh> {
        let dx = start.xz().distance(end.xz());
        let dy = start.y - end.y;
        // Re-parameterize to make it easier to control. Invert because at low tension values
        // the curve droops to negative infinity, scale by dx so that the curve looks the same as
        // you move the endpoints apart.
        let tension = dx / sag;

        // No direct formula to figure out where to put the two points on the curve to match
        // differences in height, approximate with Newton's method.
        let mut x_off = -dx / 2.0;
        let error = |x_off| catenary(x_off, tension) - catenary(x_off + dx, tension) - dy;
        for _ in 0..Self::NEWTON_ITERS {
            let d_error = catenary_dx(x_off, tension) - catenary_dx(x_off + dx, tension);
            x_off -= error(x_off) / d_error;
        }
        let x_off = x_off;
        if x_off.is_nan() || error(x_off).abs() > Self::MAX_ERR {
            return Line::build_straight_line(start, end, segments);
        }

        let y_off = start.y - catenary(x_off, tension);

        let position = |i| match i {
            0 => start,
            i if i == segments => end,
            i => {
                let t = (i as f32) / (segments as f32);
                let xz = start.xz().lerp(end.xz(), t);
                let y = catenary((t * dx) + x_off, tension) + y_off;
                Vec3::new(xz.x, y, xz.y)
            }
        };
        let dxz = end.xz() - start.xz();
        let tangent = |i| {
            let t = (i as f32) / (segments as f32);
            // Partial derivative dy/dt of the curve
            let y = dx * catenary_dx((t * dx) + x_off, tension);
            Vec3::new(dxz.x, y, dxz.y).normalize()
        };
        let normal = |i| Vec3::Y.reject_from_normalized(tangent(i));
        Line::build_with_normals(&position, &normal, &tangent, segments)
    }
}

/// Golden ratio, Phi, `(1 + 5.sqrt())/2`
const PHI: f32 = 1.618_034;
/// An Icosahedron, a regular 20-sided convex polyhedra. Useful for approximating spheres.
pub struct Icosahedron;
impl Icosahedron {
    const VERTS: [(f32, f32, f32); 12] = [
        (0., PHI, 1.),
        (0., PHI, -1.),
        (PHI, 1., 0.),
        (-PHI, 1., 0.),
        (1., 0., -PHI),
        (-1., 0., -PHI),
        (-1., 0., PHI),
        (1., 0., PHI),
        (PHI, -1., 0.),
        (-PHI, -1., 0.),
        (0., -PHI, -1.),
        (0., -PHI, 1.),
    ];
    const FACES: [[usize; 3]; 20] = [
        [0, 1, 3],
        [0, 2, 1],
        [1, 2, 4],
        [1, 5, 3],
        [1, 4, 5],
        [0, 3, 6],
        [0, 7, 2],
        [0, 6, 7],
        [2, 8, 4],
        [2, 7, 8],
        [3, 9, 6],
        [3, 5, 9],
        [4, 10, 5],
        [6, 11, 7],
        [4, 8, 10],
        [5, 10, 9],
        [6, 9, 11],
        [7, 11, 8],
        [8, 11, 10],
        [9, 10, 11],
    ];
    pub fn build(center: Vec3, radius: f32) -> Result<HalfEdgeMesh> {
        // Verts aren't at radius 1, correct for that here
        let radius = radius / (PHI * PHI + 1.).sqrt();
        let verts = Self::VERTS
            .iter()
            .map(|(x, y, z)| (Vec3::new(*x, *y, *z) * radius) + center)
            .collect_vec();
        HalfEdgeMesh::build_from_polygons(&verts, &Self::FACES)
    }
}

#[blackjack_macros::blackjack_lua_module]
mod lua_api {
    use super::*;
    use crate::lua_engine::lua_stdlib::LVec3;

    /// Creates a box with given `center` and `size` vectors.
    #[lua(under = "Primitives")]
    fn cube(center: LVec3, size: LVec3) -> Result<HalfEdgeMesh> {
        Box::build(center.0, size.0)
    }

    /// Creates a single quad, located at `center` and oriented along its
    /// `normal` and `right` vectors with given `size`.
    #[lua(under = "Primitives")]
    fn quad(center: LVec3, normal: LVec3, right: LVec3, size: LVec3) -> Result<HalfEdgeMesh> {
        Quad::build(center.0, normal.0, right.0, size.0.truncate())
    }

    /// Creates an open circle (polyline) with given `center`, `radius` and
    /// `num_vertices`.
    #[lua(under = "Primitives")]
    fn circle(center: LVec3, radius: f32, num_vertices: f32, filled: bool) -> Result<HalfEdgeMesh> {
        if filled {
            Circle::build(center.0, radius, num_vertices as usize)
        } else {
            Circle::build_open(center.0, radius, num_vertices as usize)
        }
    }

    /// Creates a truncated cone with the given `center`, `bottom_radius`, `top_radius`,
    /// `height`, and `num_vertices` around its radius. A `top_radius` of 0 will make a standard cone.
    #[lua(under = "Primitives")]
    fn cone(
        center: LVec3,
        bottom_radius: f32,
        top_radius: f32,
        height: f32,
        num_vertices: f32,
    ) -> Result<HalfEdgeMesh> {
        Cone::build(
            center.0,
            top_radius,
            bottom_radius,
            height,
            num_vertices as usize,
        )
    }

    /// Creates a cylinder with the given `center`, `radius`, `height`, and `num_vertices around its radius`.
    #[lua(under = "Primitives")]
    fn cylinder(
        center: LVec3,
        radius: f32,
        height: f32,
        num_vertices: f32,
    ) -> Result<HalfEdgeMesh> {
        Cylinder::build(center.0, radius, height, num_vertices as usize)
    }

    /// Creates a UV-sphere with given `center` and `radius`. The `rings` and
    /// `segments` let you specify the number of longitudinal
    /// and vertical sections respectively.
    #[lua(under = "Primitives")]
    fn uv_sphere(center: LVec3, radius: f32, segments: u32, rings: u32) -> Result<HalfEdgeMesh> {
        UVSphere::build(center.0, segments, rings, radius)
    }

    /// Creates an Icosahedron with given `center` and `radius`, a regular polyhedra useful for approximating spheres
    /// without artifacts around the poles.
    #[lua(under = "Primitives")]
    fn icosahedron(center: LVec3, radius: f32) -> Result<HalfEdgeMesh> {
        Icosahedron::build(center.0, radius)
    }

    /// Creates a polyline with `start` and `end` points split into a number of
    /// `segments`.
    #[lua(under = "Primitives")]
    fn line(start: LVec3, end: LVec3, segments: u32) -> Result<HalfEdgeMesh> {
        Line::build_straight_line(start.0, end.0, segments)
    }

    /// Creates a polyline from a given sequence of `points`.
    #[lua(under = "Primitives")]
    fn line_from_points(points: Vec<LVec3>) -> Result<HalfEdgeMesh> {
        Line::build_from_points(LVec3::cast_vector(points))
    }

    /// Creates a polyline from a given sequence of `points` with normals.
    #[lua(under = "Primitives")]
    fn line_with_normals(
        points: Vec<LVec3>,
        normals: Vec<LVec3>,
        tangents: Vec<LVec3>,
        segments: u32,
    ) -> Result<HalfEdgeMesh> {
        let pts = LVec3::cast_vector(points);
        let nrms = LVec3::cast_vector(normals);
        let tgts = LVec3::cast_vector(tangents);
        let position = |i: u32| pts[i as usize];
        let normal = |i: u32| nrms[i as usize];
        let tangent = |i: u32| tgts[i as usize];
        Line::build_with_normals(&position, &normal, &tangent, segments)
    }

    /// Creates a catenary curve, the curve followed by a chain or rope hanging between two points,
    /// between `start` and `end` split into a number of `segments`. `sag` adjusts how much the curve sags,
    /// higher values make the curve hang lower, lower values make it closer to a straight line.
    #[lua(under = "Primitives")]
    fn catenary(start: LVec3, end: LVec3, sag: f32, segments: u32) -> Result<HalfEdgeMesh> {
        Catenary::build(start.0, end.0, sag, segments)
    }

    /// Creates a single polygon from a given set of points.
    #[lua(under = "Primitives")]
    fn polygon(points: Vec<LVec3>) -> Result<HalfEdgeMesh> {
        Polygon::build_from_points(LVec3::cast_vector(points))
    }

    ///Creates a point cloud arranged in a grid
    #[lua(under = "Primitives")]
    fn grid(x: u32, y: u32, spacing_x: f32, spacing_y: f32) -> Result<HalfEdgeMesh> {
        Grid::build(x, y, spacing_x, spacing_y)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_cone() {
        let cone = Cone::build(Vec3::ZERO, 0.0, 1.0, 1.0, 8).unwrap();
        assert_eq!(cone.read_connectivity().num_vertices(), 9);

        Cone::build(Vec3::ZERO, 1.0, 2.0, 1.0, 8).unwrap();
        Cone::build_cone(Vec3::ZERO, 1.0, 1.0, 8).unwrap();
        Cone::build_truncated_cone(Vec3::ZERO, 1.0, 2.0, 1.0, 8).unwrap();
    }

    #[test]
    fn test_cylinder() {
        Cylinder::build(Vec3::ZERO, 1.0, 1.0, 8).unwrap();
    }

    #[test]
    fn test_circle() {
        Circle::build(Vec3::ZERO, 1.0, 24).unwrap();
        Circle::build(Vec3::ZERO, 1.0, 3).unwrap();

        // Not enough vertices to make a circle
        assert!(Circle::build(Vec3::ZERO, 1.0, 2).is_err());
        assert!(Circle::build(Vec3::ZERO, 1.0, 0).is_err());
    }

    #[test]
    fn test_catenary() {
        let start = Vec3::ZERO;
        let end = Vec3::new(0.0, 1.0, 1.0);
        let curve = Catenary::build(start, end, 1.0, 8).unwrap();
        assert_eq!(curve.read_connectivity().num_vertices(), 9);
        let pos = curve.read_positions();
        // Want to have the exact endpoints and not ones computed from the curve.
        assert!(pos.iter().map(|x| x.1).contains(&start));
        assert!(pos.iter().map(|x| x.1).contains(&end));
    }

    #[test]
    fn test_line_from_points() {
        // Too few points can cause problems with normal/tangent calculations
        Line::build_from_points(vec![]).unwrap();
        Line::build_from_points(vec![Vec3::ZERO]).unwrap();
        Line::build_from_points(vec![Vec3::ZERO, Vec3::Y]).unwrap();
    }

    #[test]
    fn test_icosahedron() {
        Icosahedron::build(Vec3::ZERO, 1.).unwrap();
    }
}
