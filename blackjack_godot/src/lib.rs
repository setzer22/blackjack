use blackjack_engine::graph_compiler::BlackjackGameAsset;
use blackjack_engine::lua_engine::LuaRuntime;
use blackjack_engine::mesh::halfedge::HalfEdgeMesh;
use blackjack_engine::prelude::*;
use gdnative::api as gd;
use gdnative::prelude::*;

use anyhow::{anyhow, bail, Result};

#[derive(NativeClass)]
#[inherit(Node)]
pub struct BlackjackAsset {
    asset: BlackjackGameAsset,
    lua_runtime: LuaRuntime,
    child_mesh_instance: Option<Ref<gd::MeshInstance>>,
}

fn halfedge_to_godot_mesh(mesh: &HalfEdgeMesh) -> Result<Ref<gd::ArrayMesh>> {
    let mut gd_verts = PoolArray::<Vector3>::new();
    let mut gd_uvs = PoolArray::<Vector2>::new();
    let mut gd_normals = PoolArray::<Vector3>::new();
    let mut gd_indices = PoolArray::<i32>::new();

    // TODO: @perf To preserve UVs, for now we just duplicate all vertices for
    // each of their incident faces and generate disconnected polygons.

    let conn = mesh.read_connectivity();
    let positions = mesh.read_positions();
    let normals = mesh.read_vertex_normals(); // TODO: No face normal support for now
    let uvs = mesh.read_uvs();

    for (idx, (h_id, _)) in conn.iter_halfedges().enumerate() {
        // Index -- TODO: Dummy
        gd_indices.push(idx as i32);

        // Position
        let v_id = conn.at_halfedge(h_id).vertex().try_end()?;
        let v_pos = positions[v_id];
        gd_verts.push(Vector3::new(v_pos.x, v_pos.y, v_pos.z));

        // UV
        if let Some(uvs) = uvs.as_ref() {
            let uv = uvs[h_id];
            gd_uvs.push(Vector2::new(uv.x, uv.y));
        }

        // Normal
        if let Some(normals) = normals.as_ref() {
            let normal = normals[v_id];
            gd_normals.push(Vector3::new(normal.x, normal.y, normal.z));
        }
    }

    let arr = VariantArray::new();
    arr.resize(gd::Mesh::ARRAY_MAX as i32);
    arr.set(gd::Mesh::ARRAY_VERTEX as i32, gd_verts);
    arr.set(gd::Mesh::ARRAY_TEX_UV as i32, gd_uvs);
    arr.set(gd::Mesh::ARRAY_NORMAL as i32, gd_normals);
    arr.set(gd::Mesh::ARRAY_INDEX as i32, gd_indices);

    let mesh = gd::ArrayMesh::new();
    mesh.add_surface_from_arrays(
        gd::Mesh::PRIMITIVE_TRIANGLES,
        arr.into_shared(),
        VariantArray::new_shared(),
        gd::Mesh::ARRAY_COMPRESS_DEFAULT,
    );

    Ok(mesh.into_shared())
}

#[methods]
impl BlackjackAsset {
    fn new(_owner: &Node) -> Self {
        let reader =
            std::io::BufReader::new(std::fs::File::open("/home/josep/capsule.bga").unwrap());
        let asset: BlackjackGameAsset = ron::de::from_reader(reader).unwrap();

        BlackjackAsset {
            asset,
            // TODO: Gotta send a path to `initialize`
            lua_runtime: LuaRuntime::initialize().unwrap(),
            child_mesh_instance: None,
        }
    }

    #[export]
    fn _ready(&self, _owner: &Node) {
        godot_print!("Hello, world.");
    }

    #[export]
    fn _process(&self, _owner: &Node, _delta: f64) {
        let mesh = blackjack_engine::lua_engine::run_program(
            &self.lua_runtime.lua,
            &self.asset.program.lua_program,
            &self.asset.params,
        ).unwrap();
        let godot_mesh = halfedge_to_godot_mesh(&mesh).unwrap();
        let child = unsafe { self.child_mesh_instance.unwrap().assume_safe() };
        child.set_mesh(godot_mesh);
    }
}

fn init(handle: InitHandle) {
    handle.add_class::<BlackjackAsset>();
}

godot_init!(init);
