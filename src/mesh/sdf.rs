use tessellation::nalgebra as na;

use crate::prelude::HalfEdgeMesh;

struct Sphere {
    position: na::Point3<f32>,
    radius: f32,
}
impl Sphere {
    pub fn bbox(&self) -> tessellation::BoundingBox<f32> {
        let rv = na::Vector3::<f32>::from_element(self.radius);
        tessellation::BoundingBox {
            min: na::Point3::from(self.position - rv),
            max: na::Point3::from(self.position + rv),
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

fn point_cloud_to_halfedge(
    points: impl Iterator<Item = glam::Vec3>,
    sizes: impl Iterator<Item = f32>,
) -> HalfEdgeMesh {
    let spheres = points
        .iter()
        .zip(sizes.iter())
        .map(|point, size| Sphere {
            position: na::Vector3::new(point.x, point.y, point.z),
            radius: size,
        })
        .collect_vec();

    let mut mesh = HalfEdgeMesh::build_from_polygons(positions, polygons);
}
