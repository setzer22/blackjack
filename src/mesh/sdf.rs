use itertools::Itertools;
use tessellation::nalgebra as na;

use crate::prelude::{transmute_vec, HalfEdgeMesh, VertexId};
use anyhow::Result;

struct Sphere {
    position: na::Point3<f32>,
    radius: f32,
}
impl Sphere {
    pub fn bbox(&self) -> tessellation::BoundingBox<f32> {
        let rv = na::Vector3::<f32>::from_element(self.radius);
        tessellation::BoundingBox {
            min: self.position - rv,
            max: self.position + rv,
        }
    }
}

struct SphereCloud {
    bbox: tessellation::BoundingBox<f32>,
    spheres: Vec<Sphere>,
}

impl SphereCloud {
    fn new(spheres: Vec<Sphere>) -> Self {
        let empty_bbox = tessellation::BoundingBox::new(
            &na::Point3::new(0.0, 0.0, 0.0),
            &na::Point3::new(0.0, 0.0, 0.0),
        );
        Self {
            bbox: spheres
                .iter()
                .fold(empty_bbox, |acc, sphere| acc.union(&sphere.bbox())),
            spheres,
        }
    }
}

impl tessellation::ImplicitFunction<f32> for SphereCloud {
    fn bbox(&self) -> &tessellation::BoundingBox<f32> {
        &self.bbox
    }
    fn value(&self, p: &na::Point3<f32>) -> f32 {
        // Sdf union is the min of all sdfs
        let mut min_val = f32::INFINITY;
        for sphere in &self.spheres {
            let v = sphere.position - p;
            min_val = min_val.min(na::Vector3::new(v.x, v.y, v.z).norm() - sphere.radius)
        }
        min_val
    }
    fn normal(&self, p: &na::Point3<f32>) -> na::Vector3<f32> {
        let mut normal = na::Vector3::<f32>::new(0.0, 0.0, 0.0);
        for sphere in &self.spheres {
            let v = sphere.position - p;
            normal += v.normalize() * (1.0 / v.norm());
        }
        normal.normalize()
    }
}

#[test]
pub fn test() {
    let cloud = SphereCloud::new(vec![
        Sphere {
            position: na::Point3::new(0.0, 0.0, 0.0),
            radius: 3.0,
        },
        Sphere {
            position: na::Point3::new(2.0, 0.0, 0.0),
            radius: 3.0,
        },
    ]);

    let mut mdc = tessellation::ManifoldDualContouring::new(&cloud, 0.2, 0.1);
    let triangles = mdc.tessellate().unwrap();
    dbg!(triangles);
}

pub fn point_cloud_to_halfedge(mesh: &HalfEdgeMesh) -> Result<HalfEdgeMesh> {
    let positions = mesh.read_positions();
    let sizes = mesh
        .channels
        .read_channel_by_name::<VertexId, f32>("size")?;

    let spheres = mesh
        .read_connectivity()
        .iter_vertices()
        .map(|(v, _)| {
            let point = positions[v];
            let size = sizes[v];
            Sphere {
                position: na::Point3::new(point.x, point.y, point.z),
                radius: size,
            }
        })
        .collect_vec();

    let cloud = SphereCloud::new(spheres);

    let mut mdc = tessellation::ManifoldDualContouring::new(&cloud, 0.05, 0.1);
    let triangles = mdc
        .tessellate()
        .ok_or_else(|| anyhow::anyhow!("Failed to tessellate sdf"))?;

    // SAFETY: Vec3 and [f32;3] have the exact same layout and both are Copy
    let positions = unsafe { transmute_vec::<_, glam::Vec3>(triangles.vertices) };

    HalfEdgeMesh::build_from_polygons(&positions, &triangles.faces)
}
