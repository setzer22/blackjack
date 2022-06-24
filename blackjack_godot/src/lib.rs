use blackjack_engine::graph::BlackjackValue;
use blackjack_engine::graph::InputValueConfig;
use blackjack_engine::graph_compiler::BlackjackGameAsset;
use blackjack_engine::graph_compiler::ExternalParamAddr;
use blackjack_engine::lua_engine::LuaRuntime;
use blackjack_engine::mesh::halfedge::HalfEdgeMesh;
use blackjack_engine::prelude::selection::SelectionExpression;
use blackjack_engine::prelude::*;
use gdnative::api as gd;
use gdnative::prelude::*;

use anyhow::{anyhow, bail, Result};

#[macro_use]
mod helpers;
use helpers::*;

#[derive(NativeClass)]
#[inherit(Node)]
pub struct BlackjackAsset {
    asset: BlackjackGameAsset,
    lua_runtime: LuaRuntime,
    child_mesh_instance: Option<Ref<gd::MeshInstance>>,
    needs_update: bool,
}

preload!(SCALAR_PROP_SCN, PackedScene, "res://ScalarProp.tscn");
preload!(VECTOR_PROP_SCN, PackedScene, "res://VectorProp.tscn");
preload!(SELECTION_PROP_SCN, PackedScene, "res://SelectionProp.tscn");
preload!(STRING_PROP_SCN, PackedScene, "res://StringProp.tscn");

#[methods]
impl BlackjackAsset {
    fn new(_owner: &Node) -> Self {
        let reader =
            std::io::BufReader::new(std::fs::File::open("/home/josep/promoted_test.bga").unwrap());
        let asset: BlackjackGameAsset = ron::de::from_reader(reader).unwrap();

        BlackjackAsset {
            asset,
            // TODO: Gotta send a path to `initialize`
            lua_runtime: LuaRuntime::initialize().unwrap(),
            child_mesh_instance: None,
            needs_update: true,
        }
    }

    #[export]
    fn on_param_changed(
        &mut self,
        _owner: &Node,
        new_value: Variant,
        param_name: String,
    ) -> Option<()> {
        if let Some(value) = self.asset.params.get_mut(&ExternalParamAddr(param_name)) {
            match &mut value.value {
                blackjack_engine::graph::BlackjackValue::Vector(v) => {
                    let new_v = new_value.try_to::<Vector3>().ok()?;
                    *v = Vec3::new(new_v.x, new_v.y, new_v.z);
                }
                blackjack_engine::graph::BlackjackValue::Scalar(s) => {
                    let new_s = new_value.try_to::<f32>().ok()?;
                    *s = new_s;
                }
                blackjack_engine::graph::BlackjackValue::String(s) => {
                    let new_s = new_value.try_to::<String>().ok()?;
                    *s = new_s;
                }
                blackjack_engine::graph::BlackjackValue::Selection(text, sel) => {
                    let new_s = new_value.try_to::<String>().ok()?;
                    if let Ok(new_sel) = SelectionExpression::parse(&new_s) {
                        *text = new_s;
                        *sel = Some(new_sel);
                    } else {
                        *text = new_s;
                        *sel = None;
                    }
                }
                blackjack_engine::graph::BlackjackValue::None => {}
            }
            self.needs_update = true;
        }
        Some(())
    }

    fn generate_params_gui(&self, owner: TRef<Node>) -> Ref<Control> {
        let ui = unsafe { gd::PanelContainer::new().into_shared().assume_safe() };
        let scroll = unsafe { gd::ScrollContainer::new().into_shared().assume_safe() };
        let vbox = unsafe { gd::VBoxContainer::new().into_shared().assume_safe() };

        ui.add_child(scroll, true);
        scroll.add_child(vbox, true);
        scroll.set_enable_h_scroll(false);
        ui.set_size(Vector2::new(80.0, 600.0), false);

        for (param_addr, value) in self.asset.params.iter() {
            if let Some(param_name) = &value.promoted_name {
                let prop = match (&value.config, &value.value) {
                    (_, BlackjackValue::Vector(v)) => {
                        let prop = instance_preloaded::<Control, _>(
                            VECTOR_PROP_SCN.clone(),
                            vbox.as_ref(),
                        );
                        gdcall!(prop, init, param_name, Vector3::new(v.x, v.y, v.z));
                        prop
                    }
                    (InputValueConfig::Scalar { min, max, .. }, BlackjackValue::Scalar(s)) => {
                        let prop = instance_preloaded::<Control, _>(
                            SCALAR_PROP_SCN.clone(),
                            vbox.as_ref(),
                        );
                        gdcall!(prop, init, param_name, s, min, max);
                        prop
                    }
                    (_, BlackjackValue::String(s)) => {
                        let prop = instance_preloaded::<Control, _>(
                            STRING_PROP_SCN.clone(),
                            vbox.as_ref(),
                        );
                        gdcall!(prop, init, param_name, s);
                        prop
                    }
                    (_, BlackjackValue::Selection(_, s)) => {
                        let prop = instance_preloaded::<Control, _>(
                            SELECTION_PROP_SCN.clone(),
                            vbox.as_ref(),
                        );
                        gdcall!(
                            prop,
                            init,
                            param_addr.0,
                            s.clone().unwrap_or(SelectionExpression::None).unparse()
                        );
                        prop
                    }
                    // TODO: For now this ignore any malformed parameters.
                    _ => continue,
                };
                prop.connect(
                    "on_changed",
                    owner,
                    "on_param_changed",
                    VariantArray::from_iter(&[param_addr.0.to_variant()]).into_shared(),
                    0,
                )
                .expect("Failed to connect signal");
            }
        }

        ui.upcast::<Control>().claim()
    }

    #[export]
    fn _ready(&mut self, owner: TRef<Node>) {
        godot_print!("Hello, world.");
        let child_mesh_instance = gd::MeshInstance::new().into_shared();
        owner.add_child(unsafe { child_mesh_instance.assume_safe() }, false);
        self.child_mesh_instance = Some(child_mesh_instance);

        let child_gui = self.generate_params_gui(owner);
        owner.add_child(child_gui, true);
    }

    #[export]
    fn _process(&self, _owner: &Node, _delta: f64) {
        if self.needs_update {
            let mesh = blackjack_engine::lua_engine::run_program(
                &self.lua_runtime.lua,
                &self.asset.program.lua_program,
                &self.asset.params,
            )
            .unwrap();
            let godot_mesh = halfedge_to_godot_mesh(&mesh).unwrap();
            let child = unsafe { self.child_mesh_instance.unwrap().assume_safe() };
            child.set_mesh(godot_mesh);
        }
    }
}

/// Converts a Blackjack HalfEdgeMesh into a Godot ArrayMesh
fn halfedge_to_godot_mesh(mesh: &HalfEdgeMesh) -> Result<Ref<gd::ArrayMesh>> {
    let mut gd_verts = PoolArray::<Vector3>::new();
    let mut gd_uvs = PoolArray::<Vector2>::new();
    let mut gd_normals = PoolArray::<Vector3>::new();
    let mut gd_indices = PoolArray::<i32>::new();

    let conn = mesh.read_connectivity();
    let positions = mesh.read_positions();
    let normals = mesh.read_vertex_normals(); // TODO: No face normal support for now
    let uvs = mesh.read_uvs();

    let mut counter = 0;
    for (f_id, _) in conn.iter_faces() {
        let face_halfedges = conn.face_edges(f_id);
        // NOTE: Iterate halfedges in reverse order because godot uses the other
        // winding direction.
        for h_id in face_halfedges.iter_cpy().rev() {
            let v_id = conn.at_halfedge(h_id).vertex().try_end()?;

            // Position
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

        // Indices. Simple fan triangulation using the face vertices.
        let i0 = counter;
        for (i1, i2) in (counter + 1..counter + face_halfedges.len()).tuple_windows() {
            gd_indices.push(i0 as i32);
            gd_indices.push(i1 as i32);
            gd_indices.push(i2 as i32);
        }
        counter += face_halfedges.len();
    }

    let arr = VariantArray::new();
    arr.resize(gd::Mesh::ARRAY_MAX as i32);
    arr.set(gd::Mesh::ARRAY_VERTEX as i32, gd_verts);
    if uvs.is_some() {
        arr.set(gd::Mesh::ARRAY_TEX_UV as i32, gd_uvs);
    }
    if normals.is_some() {
        arr.set(gd::Mesh::ARRAY_NORMAL as i32, gd_normals);
    }
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

fn init(handle: InitHandle) {
    handle.add_class::<BlackjackAsset>();
}

godot_init!(init);
