use std::{cell::RefCell, rc::Rc};

use mlua::{Function, Value};

use crate::prelude::halfedge::{DynChannel, RawChannelId};

use super::*;

pub fn load(lua: &Lua) -> anyhow::Result<()> {
    let globals = lua.globals();
    let ops = lua.create_table()?;
    globals.set("Ops", ops.clone())?;

    lua_fn!(lua, ops, "chamfer", |vertices: SelectionExpression,
                                  amount: f32,
                                  mesh: AnyUserData|
     -> () {
        let mesh = mesh.borrow_mut::<HalfEdgeMesh>()?;
        mesh.write_connectivity().clear_debug();
        let verts = mesh
            .read_connectivity()
            .resolve_vertex_selection_full(vertices);
        for v in verts {
            crate::mesh::halfedge::edit_ops::chamfer_vertex(
                &mut mesh.write_connectivity(),
                &mut mesh.write_positions(),
                v,
                amount,
            )
            .map_lua_err()?;
        }
        Ok(())
    });

    lua_fn!(lua, ops, "bevel", |edges: SelectionExpression,
                                amount: f32,
                                mesh: AnyUserData|
     -> () {
        let result = mesh.borrow_mut::<HalfEdgeMesh>()?;
        {
            let edges = result
                .read_connectivity()
                .resolve_halfedge_selection_full(edges);
            crate::mesh::halfedge::edit_ops::bevel_edges(
                &mut result.write_connectivity(),
                &mut result.write_positions(),
                &edges,
                amount,
            )
            .map_lua_err()?;
        }
        Ok(())
    });

    lua_fn!(lua, ops, "extrude", |faces: SelectionExpression,
                                  amount: f32,
                                  mesh: AnyUserData|
     -> () {
        let result = mesh.borrow_mut::<HalfEdgeMesh>()?;
        {
            let faces = result
                .read_connectivity()
                .resolve_face_selection_full(faces);
            crate::mesh::halfedge::edit_ops::extrude_faces(
                &mut result.write_connectivity(),
                &mut result.write_positions(),
                &faces,
                amount,
            )
            .map_lua_err()?;
        }
        Ok(())
    });

    lua_fn!(lua, ops, "merge", |a: AnyUserData, b: AnyUserData| -> () {
        let mut a = a.borrow_mut::<HalfEdgeMesh>()?;
        let b = b.borrow::<HalfEdgeMesh>()?;
        a.merge_with(&b);
        Ok(())
    });

    lua_fn!(lua, ops, "subdivide", |mesh: AnyUserData,
                                    iterations: usize,
                                    catmull_clark: bool|
     -> HalfEdgeMesh {
        let mesh = &mesh.borrow::<HalfEdgeMesh>()?;
        let new_mesh = CompactMesh::<false>::from_halfedge(mesh).map_lua_err()?;
        Ok(new_mesh
            .subdivide_multi(iterations, catmull_clark)
            .to_halfedge())
    });

    lua_fn!(lua, ops, "set_smooth_normals", |mesh: AnyUserData| -> () {
        let mut mesh = mesh.borrow_mut::<HalfEdgeMesh>()?;
        crate::mesh::halfedge::edit_ops::set_smooth_normals(&mut mesh).map_lua_err()?;
        Ok(())
    });

    lua_fn!(lua, ops, "set_flat_normals", |mesh: AnyUserData| -> () {
        let mut mesh = mesh.borrow_mut::<HalfEdgeMesh>()?;
        crate::mesh::halfedge::edit_ops::set_flat_normals(&mut mesh).map_lua_err()?;
        Ok(())
    });

    lua_fn!(lua, ops, "bridge_loops", |mesh: AnyUserData,
                                       loop_1: SelectionExpression,
                                       loop_2: SelectionExpression|
     -> () {
        let mut mesh = mesh.borrow_mut::<HalfEdgeMesh>()?;
        let loop_1 = mesh
            .read_connectivity()
            .resolve_halfedge_selection_full(dbg!(loop_1));
        let loop_2 = mesh
            .read_connectivity()
            .resolve_halfedge_selection_full(dbg!(loop_2));
        crate::mesh::halfedge::edit_ops::bridge_loops(&mut mesh, &loop_1, &loop_2).map_lua_err()?;
        Ok(())
    });

    let types = lua.create_table()?;
    types.set("VertexId", ChannelKeyType::VertexId)?;
    types.set("FaceId", ChannelKeyType::FaceId)?;
    types.set("HalfEdgeId", ChannelKeyType::HalfEdgeId)?;
    types.set("Vec3", ChannelValueType::Vec3)?;
    types.set("f32", ChannelValueType::f32)?;
    globals.set("Types", types)?;

    Ok(())
}

fn mesh_reduce<'lua>(
    mesh: &HalfEdgeMesh,
    kty: ChannelKeyType,
    init: Value<'lua>,
    f: Function<'lua>,
) -> mlua::Result<Value<'lua>> {
    let mut acc = init;
    let conn = mesh.read_connectivity();
    match kty {
        ChannelKeyType::VertexId => {
            for (id, _) in conn.iter_vertices() {
                acc = f.call((acc, id))?;
            }
        }
        ChannelKeyType::FaceId => {
            for (id, _) in conn.iter_faces() {
                acc = f.call((acc, id))?;
            }
        }
        ChannelKeyType::HalfEdgeId => {
            for (id, _) in conn.iter_halfedges() {
                acc = f.call((acc, id))?;
            }
        }
    }
    Ok(acc)
}

fn mesh_channel_to_lua_table<'lua>(
    lua: &'lua Lua,
    mesh: &HalfEdgeMesh,
    kty: ChannelKeyType,
    vty: ChannelValueType,
    ch_id: RawChannelId,
) -> mlua::Result<mlua::Table<'lua>> {
    use slotmap::Key;
    let conn = mesh.read_connectivity();
    let keys: Box<dyn Iterator<Item = u64>> = match kty {
        ChannelKeyType::VertexId => {
            Box::new(conn.iter_vertices().map(|(v_id, _)| v_id.data().as_ffi()))
        }
        ChannelKeyType::FaceId => Box::new(conn.iter_faces().map(|(f_id, _)| f_id.data().as_ffi())),
        ChannelKeyType::HalfEdgeId => {
            Box::new(conn.iter_halfedges().map(|(h_id, _)| h_id.data().as_ffi()))
        }
    };
    Ok(mesh
        .channels
        .dyn_read_channel(kty, vty, ch_id)
        .map_lua_err()?
        .to_table(keys, lua))
}

impl UserData for HalfEdgeMesh {
    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method(
            "get_channel",
            |lua, this, (kty, vty, name): (ChannelKeyType, ChannelValueType, String)| {
                let ch_id = this
                    .channels
                    .channel_id_dyn(kty, vty, &name)
                    .ok_or_else(|| anyhow::anyhow!("Channel '{name}' not found"))
                    .map_lua_err()?;
                mesh_channel_to_lua_table(lua, this, kty, vty, ch_id)
            },
        );
        methods.add_method("set_channel", |lua, this, (kty, vty, name, table)| {
            use slotmap::Key;
            let name: String = name;
            let conn = this.read_connectivity();
            let keys: Box<dyn Iterator<Item = u64>> = match kty {
                ChannelKeyType::VertexId => {
                    Box::new(conn.iter_vertices().map(|(v_id, _)| v_id.data().as_ffi()))
                }
                ChannelKeyType::FaceId => {
                    Box::new(conn.iter_faces().map(|(f_id, _)| f_id.data().as_ffi()))
                }
                ChannelKeyType::HalfEdgeId => {
                    Box::new(conn.iter_halfedges().map(|(h_id, _)| h_id.data().as_ffi()))
                }
            };
            this.channels
                .dyn_write_channel_by_name(kty, vty, &name)
                .map_lua_err()?
                .set_from_table(keys, lua, table)
                .map_lua_err()
        });
        methods.add_method_mut(
            "ensure_channel",
            |lua, this, (kty, vty, name): (ChannelKeyType, ChannelValueType, String)| {
                let id = this.channels.ensure_channel_dyn(kty, vty, &name);
                mesh_channel_to_lua_table(lua, this, kty, vty, id)
            },
        );
        methods.add_method("iter_vertices", |lua, this, ()| {
            let vertices: Vec<VertexId> = this
                .read_connectivity()
                .iter_vertices()
                .map(|(id, _)| id)
                .collect();
            let mut i = 0;
            lua.create_function_mut(move |lua, ()| {
                let val = if i < vertices.len() {
                    vertices[i].to_lua(lua)?
                } else {
                    Value::Nil
                };
                i += 1;
                Ok(val)
            })
        });
        methods.add_method("clone", |_lua, this, ()| Ok(this.clone()));

        methods.add_method(
            "reduce",
            |_lua, this, (kty, init, f): (ChannelKeyType, Value, Function)| {
                mesh_reduce(this, kty, init, f)
            },
        );

        methods.add_method(
            "reduce_vertices",
            |_lua, this, (init, f): (Value, Function)| {
                mesh_reduce(this, ChannelKeyType::VertexId, init, f)
            },
        );
        methods.add_method(
            "reduce_faces",
            |_lua, this, (init, f): (Value, Function)| {
                mesh_reduce(this, ChannelKeyType::FaceId, init, f)
            },
        );
        methods.add_method(
            "reduce_halfedges",
            |_lua, this, (init, f): (Value, Function)| {
                mesh_reduce(this, ChannelKeyType::HalfEdgeId, init, f)
            },
        );

        methods.add_method(
            "vertex_position",
            |_lua, this: &HalfEdgeMesh, v: VertexId| Ok(Vec3(this.read_positions()[v])),
        );
    }
}

pub struct SharedChannel(pub Rc<RefCell<dyn DynChannel>>);
impl Clone for SharedChannel {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

impl UserData for SharedChannel {
    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(
            mlua::MetaMethod::NewIndex,
            |lua, this, (key, val): (Value, Value)| {
                this.0.borrow_mut().set_lua(lua, key, val).map_lua_err()?;
                Ok(())
            },
        );
        methods.add_meta_method(mlua::MetaMethod::Index, |lua, this, key: Value| {
            let value = this.0.borrow().get_lua(lua, key).map_lua_err()?;
            Ok(value.clone())
        });
        methods.add_meta_method(
            mlua::MetaMethod::NewIndex,
            |lua, this, (key, val): (Value, Value)| {
                this.0.borrow_mut().set_lua(lua, key, val).map_lua_err()?;
                Ok(())
            },
        );
    }
}
