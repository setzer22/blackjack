// Copyright (C) 2022 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use gdnative::api::Material;
use slotmap::KeyData;
use slotmap::SlotMap;
use std::collections::BTreeMap;
use std::sync::atomic::AtomicBool;

use blackjack_engine::graph::BlackjackValue;
use blackjack_engine::graph::InputValueConfig;
use blackjack_engine::graph_compiler::BlackjackJackAsset;
use blackjack_engine::graph_compiler::ExternalParamAddr;
use blackjack_engine::lua_engine::LuaRuntime;
use blackjack_engine::mesh::halfedge::HalfEdgeMesh;
use blackjack_engine::prelude::selection::SelectionExpression;
use blackjack_engine::prelude::*;
use gdnative::api as gd;
use gdnative::prelude::*;

use anyhow::Result;

mod godot_lua_io;

slotmap::new_key_type! { pub struct JackId; }

impl FromVariant for JackId {
    fn from_variant(variant: &Variant) -> Result<Self, FromVariantError> {
        match variant.dispatch() {
            VariantDispatch::I64(id_data) => Ok(JackId(KeyData::from_ffi(id_data as u64))),
            _ => Err(FromVariantError::Custom("Invalid JackId value".into())),
        }
    }
}
impl ToVariant for JackId {
    fn to_variant(&self) -> Variant {
        self.0.as_ffi().to_variant()
    }
}

/// A singleton node that manages the lifetime for all the loaded jacks. This
/// node is never directly used by GDscript, which instead accesses it via the
/// [`BlackjackApi`].
///
/// This node is lazily initialized as an autoload-like structure when
/// `get_singleton` is called.
#[derive(NativeClass)]
#[no_constructor]
#[inherit(Node)]
pub struct BlackjackGodotRuntime {
    lua_runtime: LuaRuntime,
    jacks: SlotMap<JackId, Option<BlackjackJackAsset>>,
}

static LUA_NEEDS_INIT: AtomicBool = AtomicBool::new(true);

#[methods]
impl BlackjackGodotRuntime {
    fn initialize() -> Result<Self> {
        godot_print!("Loading Blackjack runtime");
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
        )?;

        Ok(Self {
            lua_runtime,
            jacks: SlotMap::with_key(),
        })
    }

    fn get_singleton() -> Option<Instance<Self>> {
        let engine = gd::Engine::godot_singleton();
        let tree_root = unsafe {
            engine
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
        } else if LUA_NEEDS_INIT.swap(false, std::sync::atomic::Ordering::Relaxed) {
            let new = Self::emplace(
                Self::initialize()
                    .map_err(|err| {
                        godot_error!("Error while loading Blackjack runtime: {err}");
                        err
                    })
                    .ok()?,
            );
            new.base().set_name("BlackjackRuntime");
            let new = new.into_shared();
            tree_root.add_child(new.clone(), false);
            Some(new)
        } else {
            None
        }
    }
}

#[derive(ToVariant)]
pub enum UpdateJackResult {
    Ok(Ref<gd::ArrayMesh>),
    Err(String),
}

/// A facade-like API exposed to GDScript.
#[derive(NativeClass)]
#[inherit(gd::Resource)]
pub struct BlackjackApi {}

#[methods]
impl BlackjackApi {
    fn new(_owner: &gd::Resource) -> Self {
        Self {}
    }

    fn with_runtime<U>(f: impl FnOnce(&mut BlackjackGodotRuntime) -> Option<U>) -> Option<U> {
        let runtime = BlackjackGodotRuntime::get_singleton()?;
        let runtime = unsafe { runtime.assume_safe() };
        runtime.map_mut(|runtime, _| f(runtime)).ok()?
    }

    #[export]
    fn ping(&self, _owner: &gd::Resource) -> String {
        "PONG".into()
    }

    #[export]
    fn make_jack(&self, _owner: &gd::Resource) -> Option<JackId> {
        Self::with_runtime(|runtime| Some(runtime.jacks.insert(None)))
    }

    #[export]
    fn set_jack(
        &mut self,
        _owner: &gd::Resource,
        jack_id: JackId,
        jack: Ref<gd::Resource>,
    ) -> Option<bool> {
        Self::with_runtime(|runtime| {
            let jack = unsafe { jack.assume_safe() };
            let contents = match jack.get("contents").dispatch() {
                VariantDispatch::GodotString(contents) => contents,
                _ => {
                    godot_error!("Could not load jack. Empty contents?");
                    return None;
                }
            };
            let loaded: BlackjackJackAsset = ron::from_str(&contents.to_string()).ok()?;
            *runtime.jacks.get_mut(jack_id)? = Some(loaded);
            Some(true)
        })
    }

    #[export]
    fn set_param(
        &mut self,
        _owner: &gd::Resource,
        jack_id: JackId,
        param_name: String,
        new_value: Variant,
    ) -> Option<bool> {
        Self::with_runtime(|runtime| {
            let jack = runtime.jacks.get_mut(jack_id)?.as_mut()?;
            let value = jack.params.get_mut(&ExternalParamAddr(param_name))?;
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
            Some(true)
        })
    }

    #[export]
    fn get_params(&mut self, _owner: &gd::Resource, jack_id: JackId) -> Option<Variant> {
        #[derive(FromVariant, ToVariant)]
        struct ScalarDef {
            label: String,
            addr: String,
            typ: String,
            val: f32,
            min: f32,
            max: f32,
        }

        #[derive(FromVariant, ToVariant)]
        struct GenericDef {
            label: String,
            addr: String,
            typ: String,
            val: Variant,
        }

        Self::with_runtime(|runtime| {
            let jack = runtime.jacks.get(jack_id)?.as_ref()?;

            #[allow(unused_mut)]
            let mut params = VariantArray::new();

            for (param_addr, value) in jack.params.iter() {
                if let Some(param_name) = &value.promoted_name {
                    let label = param_name.clone();
                    let addr = param_addr.0.clone();

                    match (&value.config, &value.value) {
                        (_, BlackjackValue::Vector(v)) => params.push(GenericDef {
                            label,
                            addr,
                            typ: "Vector".into(),
                            val: Vector3::new(v.x, v.y, v.z).to_variant(),
                        }),
                        (InputValueConfig::Scalar { min, max, .. }, BlackjackValue::Scalar(s)) => {
                            params.push(ScalarDef {
                                label,
                                addr,
                                typ: "Scalar".into(),
                                val: *s,
                                min: *min,
                                max: *max,
                            })
                        }
                        (_, BlackjackValue::String(s)) => params.push(GenericDef {
                            label,
                            addr,
                            typ: "String".into(),
                            val: s.clone().to_variant(),
                        }),
                        (_, BlackjackValue::Selection(_, s)) => params.push(GenericDef {
                            label,
                            addr,
                            typ: "Selection".into(),
                            val: s
                                .as_ref()
                                .cloned()
                                .unwrap_or(SelectionExpression::None)
                                .unparse()
                                .to_variant(),
                        }),
                        // TODO: For now this ignore any malformed parameters.
                        _ => continue,
                    }
                }
            }

            Some(params.into_shared().to_variant())
        })
    }

    #[export]
    fn update_jack(
        &mut self,
        _owner: &gd::Resource,
        jack_id: JackId,
        materials: Vec<Ref<Material>>,
    ) -> Option<UpdateJackResult> {
        Self::with_runtime(|runtime| {
            let jack = runtime.jacks.get(jack_id)?.as_ref()?;
            match blackjack_engine::lua_engine::run_program(
                &runtime.lua_runtime.lua,
                &jack.program.lua_program,
                &jack.params,
            ) {
                Ok(mesh) => {
                    let godot_mesh = halfedge_to_godot_mesh(&mesh, materials).unwrap();
                    Some(UpdateJackResult::Ok(godot_mesh))
                }
                Err(err) => Some(UpdateJackResult::Err(err.to_string())),
            }
        })
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
fn halfedge_to_godot_mesh(
    mesh: &HalfEdgeMesh,
    materials_vec: Vec<Ref<Material>>,
) -> Result<Ref<gd::ArrayMesh>> {
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
                // UV y coordinate needs to be flipped in Godot meshes.
                gd_uvs.push(Vector2::new(uv.x, -uv.y));
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

        if let Some(mat) = materials_vec.get(surface_idx as usize) {
            mesh.surface_set_material(surface_idx as i64, mat.clone());
        }
    }

    Ok(mesh.into_shared())
}

fn init(handle: InitHandle) {
    handle.add_tool_class::<BlackjackApi>();
    handle.add_tool_class::<BlackjackGodotRuntime>();
}

godot_init!(init);
