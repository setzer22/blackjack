use super::halfedge::HalfEdgeMesh;
use crate::prelude::*;

/// Loads a wavefront obj file from a given path and returns a Rend3 mesh
/// @CopyPaste from wavefront_obj.rs
pub fn load_obj_mesh(path: &str) -> r3::Mesh {
    use std::fs::File;
    use std::io::BufReader;
    use wavefront_rs::obj;
    use wavefront_rs::obj::entity::Entity;

    let mut reader = BufReader::new(File::open(path).expect("File at path"));
    let mut positions = vec![];
    let mut indices = vec![];

    obj::read_lexer::ReadLexer::read_to_end(&mut reader, |entity| match entity {
        Entity::Vertex { x, y, z, w: _w } => {
            positions.push(Vec3::new(x as f32, y as f32, z as f32));
        }
        Entity::Face { vertices } => {
            // NOTE: OBJ Wavefront indices start at 1
            let polygon: SVec<usize> = vertices.iter().map(|v| (v.vertex - 1) as usize).collect();
            // Fan triangulation
            let a = polygon[0];
            for (b, c) in polygon[1..].iter().tuple_windows() {
                indices.push(a as u32);
                indices.push(*b as u32);
                indices.push(*c as u32);
            }
        }
        _ => {}
    })
    .expect("OBJ Files in assets path should be correct");

    r3::MeshBuilder::new(positions, r3::Handedness::Left)
        .with_indices(indices)
        .build()
        .unwrap_or_else(|err| panic!("Could load mesh {}. Error: {:?}", path, err))
}

pub struct DebugMeshes {
    cylinder: r3::MeshHandle,
    sphere: r3::MeshHandle,
    base_material: r3::MaterialHandle,
    color_material_cache: HashMap<egui::Color32, r3::MaterialHandle>,
}

impl DebugMeshes {
    pub fn get_color_material(
        &mut self,
        renderer: &r3::Renderer,
        color: egui::Color32,
    ) -> r3::MaterialHandle {
        self.color_material_cache
            .entry(color)
            .or_insert_with(|| {
                let color = glam::Vec4::new(
                    color.r() as f32 / 255.0,
                    color.g() as f32 / 255.0,
                    color.b() as f32 / 255.0,
                    color.a() as f32 / 255.0,
                );
                renderer.add_material(r3::PbrMaterial {
                    albedo: r3::AlbedoComponent::Value(color),
                    ..Default::default()
                })
            })
            .clone()
    }

    pub fn get_material(
        &mut self,
        renderer: &r3::Renderer,
        mark: Option<DebugMark>,
    ) -> r3::MaterialHandle {
        if let Some(mark) = mark {
            self.get_color_material(renderer, mark.color)
        } else {
            self.base_material.clone()
        }
    }
}

pub fn add_debug_meshes(renderer: &r3::Renderer) -> DebugMeshes {
    let cylinder = renderer.add_mesh(load_obj_mesh("./assets/debug/arrow.obj"));
    let sphere = renderer.add_mesh(load_obj_mesh("./assets/debug/icosphere.obj"));
    let base_material = renderer.add_material(r3::PbrMaterial {
        albedo: r3::AlbedoComponent::Value(Vec4::ONE),
        ..Default::default()
    });

    let dbg_meshes = DebugMeshes {
        cylinder,
        sphere,
        base_material,
        color_material_cache: HashMap::new(),
    };

    // Forget about 'em! We don't want to store these and they must be alive for
    // the whole application's lifetime
    Box::leak(Box::new(dbg_meshes.cylinder.clone()));
    Box::leak(Box::new(dbg_meshes.sphere.clone()));
    Box::leak(Box::new(dbg_meshes.base_material.clone()));

    dbg_meshes
}

pub fn add_halfedge_debug(
    render_ctx: &mut RenderContext,
    debug_meshes: &mut DebugMeshes,
    mesh: &HalfEdgeMesh,
) {
    const VERTEX_THICKNESS: f32 = 0.07;
    const EDGE_THICKNESS: f32 = 0.05;
    const HALFEDGE_SEPARATION: f32 = 0.03;

    for (v_id, vertex) in mesh.iter_vertices() {
        let material =
            debug_meshes.get_material(&render_ctx.renderer, mesh.vertex_debug_mark(v_id));
        render_ctx.add_object(r3::Object {
            mesh_kind: r3::ObjectMeshKind::Static(debug_meshes.sphere.clone()),
            material,
            transform: glam::Mat4::from_translation(vertex.position)
                * glam::Mat4::from_scale(Vec3::ONE * VERTEX_THICKNESS),
        });
    }

    for (h, _) in mesh.iter_halfedges() {
        let face_centroid = mesh
            .at_halfedge(h)
            .face()
            .try_end()
            .map(|face| mesh.face_vertex_average(face));

        let (src, dst) = mesh.at_halfedge(h).src_dst_pair().unwrap();
        let src_pos = mesh.vertex_position(src);
        let dst_pos = mesh.vertex_position(dst);

        let midpoint = (src_pos + dst_pos) / 2.0;
        let delta = dst_pos - src_pos;
        let delta_dir = delta.normalize();

        let orientation = Quat::from_rotation_arc(Vec3::Y, delta_dir);
        let orientation = Mat4::from_rotation_translation(orientation, Vec3::ZERO);

        let material = debug_meshes.get_material(&render_ctx.renderer, mesh.halfedge_debug_mark(h));

        let towards_face = if let Ok(centroid) = face_centroid {
            (centroid - midpoint).normalize() * HALFEDGE_SEPARATION
        } else {
            Vec3::ZERO
        };

        render_ctx.add_object(r3::Object {
            mesh_kind: r3::ObjectMeshKind::Static(debug_meshes.cylinder.clone()),
            material,
            transform: Mat4::from_translation(midpoint + towards_face)
                * orientation
                * Mat4::from_scale(Vec3::new(EDGE_THICKNESS, delta.length(), EDGE_THICKNESS)),
        });
    }
}
