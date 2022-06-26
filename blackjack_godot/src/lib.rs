use gdnative::export::hint::EnumHint;
use gdnative::export::hint::StringHint;
use gdnative::export::user_data::MapMut;
use parking_lot::Mutex;
use slotmap::SlotMap;
use std::collections::BTreeMap;
use std::sync::Arc;

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
use once_cell::sync::Lazy;

mod inspector_plugin;

mod godot_lua_io;

slotmap::new_key_type! { pub struct AssetId; }

#[derive(NativeClass)]
#[inherit(Node)]
pub struct BlackjackGodotRuntime {
    lua_runtime: Option<LuaRuntime>,
    assets: SlotMap<AssetId, BlackjackAsset>,
}

#[methods]
impl BlackjackGodotRuntime {
    fn new(owner: &Node) -> Self {
        godot_print!("Loading lua runtime");
        let project_settings = gd::ProjectSettings::godot_singleton();
        let library_path = project_settings
            .get_setting("Blackjack/library_path")
            .try_to::<String>()
            .unwrap_or_else(|e| {
                godot_error!("Invalid path in project settings {e}");
                "".into()
            });
        let lua_runtime = LuaRuntime::initialize_custom(
            library_path,
            godot_lua_io::load_node_libraries_with_godot,
        )
        .map_err(|err| {
            godot_error!(
                "Blackjack could not find the Lua node libraries.\
                 Did you set the path in user settings? {err}"
            );
            err
        })
        .ok();

        Self {
            lua_runtime,
            assets: SlotMap::with_key(),
        }
    }

    fn get_singleton() -> Option<Instance<Self>> {
        let tree_root = unsafe {
            gd::Engine::godot_singleton()
                .get_main_loop()
                .unwrap()
                .assume_safe()
                .cast::<SceneTree>()
                .unwrap()
                .root()
                .unwrap()
                .assume_safe()
        };

        if tree_root.has_node("BlackjackRuntime") {
            let node = tree_root.get_node("BlackjackRuntime").unwrap();
            let node = unsafe { node.assume_safe() };
            if let Some(inst) = node.cast_instance::<Self>() {
                Some(inst.claim())
            } else {
                godot_error!("BlackjackRuntime singleton is not of the expected type.");
                None
            }
        } else {
            let new = Self::new_instance();
            new.base().set_name("BlackjackRuntime");
            let new = new.into_shared();
            tree_root.add_child(new.clone(), false);
            Some(new)
        }
    }
}

#[derive(NativeClass)]
#[register_with(Self::register_properties)]
#[inherit(Node)]
pub struct BlackjackAsset {
    #[property]
    asset_res: Option<Ref<gd::Resource>>,
    asset_path: String,
    asset: Option<BlackjackGameAsset>,
    child_mesh_instance: Option<Ref<gd::MeshInstance>>,
    needs_update: bool,
}

preload!(
    SCALAR_PROP_SCN,
    PackedScene,
    "res://addons/blackjack_engine_godot/ScalarProp.tscn"
);
preload!(
    VECTOR_PROP_SCN,
    PackedScene,
    "res://addons/blackjack_engine_godot/VectorProp.tscn"
);
preload!(
    SELECTION_PROP_SCN,
    PackedScene,
    "res://addons/blackjack_engine_godot/SelectionProp.tscn"
);
preload!(
    STRING_PROP_SCN,
    PackedScene,
    "res://addons/blackjack_engine_godot/StringProp.tscn"
);
preload!(
    ERROR_LABEL_SCN,
    PackedScene,
    "res://addons/blackjack_engine_godot/ErrorLabel.tscn"
);

preload!(MAT1, gd::SpatialMaterial, "res://Mat1.tres");
preload!(MAT2, gd::SpatialMaterial, "res://Mat2.tres");

#[methods]
impl BlackjackAsset {
    fn new(_owner: &Node) -> Self {
        BlackjackAsset {
            asset_res: None,
            asset_path: "".into(),
            asset: None,
            child_mesh_instance: None,
            needs_update: true,
        }
    }

    fn register_properties(builder: &ClassBuilder<Self>) {
        builder
            .property("asset_path")
            .with_default("".into())
            .with_getter(|this, _| this.asset_path.to_string())
            .with_setter(|this, owner, new_val| {
                this.asset_path = new_val;
                this.reload_asset(owner)
            })
            .with_hint(StringHint::File(EnumHint::new(vec![".bga".into()])))
            .done();
        builder.signal("mesh_gen_error").done();
        builder.signal("mesh_gen_clear_error").done();
    }

    #[export]
    fn on_param_changed(
        &mut self,
        _owner: &Node,
        new_value: Variant,
        param_name: String,
    ) -> Option<()> {
        if let Some(value) = self
            .asset
            .as_mut()
            .and_then(|asset| asset.params.get_mut(&ExternalParamAddr(param_name)))
        {
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
        ui.set_custom_minimum_size(Vector2::new(80.0, 600.0));

        if let Some(asset) = &self.asset {
            for (param_addr, value) in asset.params.iter() {
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
            let error_label =
                instance_preloaded::<Control, _>(ERROR_LABEL_SCN.clone(), vbox.as_ref());
            owner
                .connect(
                    "mesh_gen_error",
                    error_label,
                    "_on_error",
                    VariantArray::new_shared(),
                    0,
                )
                .expect("Connecting signal should not fail");
            owner
                .connect(
                    "mesh_gen_clear_error",
                    error_label,
                    "_on_clear_error",
                    VariantArray::new_shared(),
                    0,
                )
                .expect("Connecting signal should not fail");
        }

        ui.upcast::<Control>().claim()
    }

    #[export]
    fn reload_asset(&mut self, _owner: TRef<Node>) {
        // TODO: Read using godot API
        (|| -> Result<()> {
            let reader = std::io::BufReader::new(std::fs::File::open(&self.asset_path)?);
            let asset: BlackjackGameAsset = ron::de::from_reader(reader)?;
            self.asset = Some(asset);
            Ok(())
        })()
        .unwrap_or_else(|err| godot_error!("Error while loading Blackjack asset {err}"));
    }

    #[export]
    fn _ready(&mut self, owner: TRef<Node>) {
        self.reload_asset(owner);

        let child_mesh_instance = gd::MeshInstance::new().into_shared();
        owner.add_child(unsafe { child_mesh_instance.assume_safe() }, false);
        self.child_mesh_instance = Some(child_mesh_instance);

        let child_gui = self.generate_params_gui(owner);
        owner.add_child(child_gui, true);
    }

    #[export]
    fn _process(&mut self, owner: &Node, _delta: f64) {
        if self.needs_update {
            match (&self.asset, BlackjackGodotRuntime::get_singleton()) {
                (Some(asset), Some(runtime)) => {
                    let runtime = unsafe { runtime.assume_safe() };
                    runtime
                        .map_mut(|runtime, _| {
                            if let Some(lua_r) = runtime.lua_runtime.as_ref() {
                                match blackjack_engine::lua_engine::run_program(
                                    &lua_r.lua,
                                    &asset.program.lua_program,
                                    &asset.params,
                                ) {
                                    Ok(mesh) => {
                                        let godot_mesh = halfedge_to_godot_mesh(&mesh).unwrap();
                                        let child = unsafe {
                                            self.child_mesh_instance.unwrap().assume_safe()
                                        };
                                        child.set_mesh(godot_mesh);
                                        owner.emit_signal("mesh_gen_clear_error", &[]);
                                        self.needs_update = false;
                                    }
                                    Err(err) => {
                                        owner.emit_signal(
                                            "mesh_gen_error",
                                            &[err.to_string().to_variant()],
                                        );
                                        self.needs_update = false;
                                    }
                                }
                            } else {
                                godot_error!("Lua runtime failed loading")
                            }
                        })
                        .expect("Could map mut");
                }
                (_, _) => {
                    godot_error!("Unexpected error while generating mesh");
                }
            }
        }
    }
}

#[derive(Default)]
pub struct GdMeshBuffers {
    gd_verts: PoolArray<Vector3>,
    gd_uvs: PoolArray<Vector2>,
    gd_normals: PoolArray<Vector3>,
    gd_indices: PoolArray<i32>,
    counter: i32,
}

/// Converts a Blackjack HalfEdgeMesh into a Godot ArrayMesh
fn halfedge_to_godot_mesh(mesh: &HalfEdgeMesh) -> Result<Ref<gd::ArrayMesh>> {
    let mut surfaces = BTreeMap::<i32, GdMeshBuffers>::new();

    let conn = mesh.read_connectivity();
    let positions = mesh.read_positions();
    let normals = mesh.read_vertex_normals(); // TODO: No face normal support for now
    let uvs = mesh.read_uvs();
    let materials = mesh
        .channels
        .read_channel_by_name::<FaceId, f32>("material");

    for (f_id, _) in conn.iter_faces() {
        let material_idx = if let Ok(materials) = &materials {
            materials[f_id] as i32
        } else {
            0
        };
        let GdMeshBuffers {
            ref mut gd_verts,
            ref mut gd_uvs,
            ref mut gd_normals,
            ref mut gd_indices,
            ref mut counter,
        } = surfaces.entry(material_idx).or_default();

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
        let i0 = *counter;
        for (i1, i2) in (*counter + 1..*counter + face_halfedges.len() as i32).tuple_windows() {
            gd_indices.push(i0 as i32);
            gd_indices.push(i1 as i32);
            gd_indices.push(i2 as i32);
        }
        *counter += face_halfedges.len() as i32;
    }

    let mesh = gd::ArrayMesh::new();
    for (
        surface_idx,
        GdMeshBuffers {
            gd_verts,
            gd_uvs,
            gd_normals,
            gd_indices,
            counter: _,
        },
    ) in surfaces
    {
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

        mesh.add_surface_from_arrays(
            gd::Mesh::PRIMITIVE_TRIANGLES,
            arr.into_shared(),
            VariantArray::new_shared(),
            gd::Mesh::ARRAY_COMPRESS_DEFAULT,
        );

        // TODO: UGLY HACK
        match surface_idx {
            0 => {
                mesh.surface_set_material(0, MAT1.clone());
            }
            1 => {
                mesh.surface_set_material(1, MAT2.clone());
            }
            _ => {}
        }
    }

    Ok(mesh.into_shared())
}

fn init(handle: InitHandle) {
    handle.add_tool_class::<BlackjackAsset>();
    handle.add_tool_class::<BlackjackGodotRuntime>();
    handle.add_class::<inspector_plugin::BlackjackInspectorPlugin>();
}

godot_init!(init);
