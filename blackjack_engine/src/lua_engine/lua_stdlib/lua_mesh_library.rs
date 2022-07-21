// Copyright (C) 2022 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{cell::RefCell, rc::Rc};

use mlua::{Function, Value};

use crate::prelude::halfedge::{
    edit_ops, AnyTraversal, DynChannel, HalfEdgeTraversal, HalfedgeTraversalHelpers, RawChannelId,
};

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
            .resolve_vertex_selection_full(&vertices)
            .map_lua_err()?;
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
                .resolve_halfedge_selection_full(&edges)
                .map_lua_err()?;
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
            let faces = result.resolve_face_selection_full(&faces).map_lua_err()?;
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

    lua_fn!(lua, ops, "bridge_chains", |mesh: AnyUserData,
                                        loop_1: SelectionExpression,
                                        loop_2: SelectionExpression,
                                        flip: usize|
     -> () {
        let mut mesh = mesh.borrow_mut::<HalfEdgeMesh>()?;
        let loop_1 = mesh
            .resolve_halfedge_selection_full(&loop_1)
            .map_lua_err()?;
        let loop_2 = mesh
            .resolve_halfedge_selection_full(&loop_2)
            .map_lua_err()?;

        crate::mesh::halfedge::edit_ops::bridge_chains_ui(&mut mesh, &loop_1, &loop_2, flip)
            .map_lua_err()?;
        Ok(())
    });

    lua_fn!(lua, ops, "make_quad", |mesh: AnyUserData,
                                    a: SelectionExpression,
                                    b: SelectionExpression,
                                    c: SelectionExpression,
                                    d: SelectionExpression|
     -> () {
        let mesh = mesh.borrow_mut::<HalfEdgeMesh>()?;

        macro_rules! get_selection {
            ($sel:expr) => {
                mesh.resolve_vertex_selection_full(&a)
                    .map_lua_err()?
                    .get(0)
                    .copied()
                    .ok_or_else(|| anyhow::anyhow!("Empty selection"))
                    .map_lua_err()?
            };
        }

        let a = get_selection!(a);
        let b = get_selection!(b);
        let c = get_selection!(c);
        let d = get_selection!(d);

        crate::mesh::halfedge::edit_ops::make_quad(&mut mesh.write_connectivity(), &[a, b, c, d])
            .map_lua_err()?;
        Ok(())
    });

    lua_fn!(lua, ops, "transform", |mesh: AnyUserData,
                                    translate: LVec3,
                                    rotate: LVec3,
                                    scale: LVec3|
     -> () {
        let mut mesh = mesh.borrow_mut::<HalfEdgeMesh>()?;
        crate::mesh::halfedge::edit_ops::transform(&mut mesh, translate.0, rotate.0, scale.0)
            .map_lua_err()?;
        Ok(())
    });

    lua_fn!(lua, ops, "make_group", |mesh: AnyUserData,
                                     key_type: ChannelKeyType,
                                     selection: SelectionExpression,
                                     group_name: String|
     -> () {
        let mut mesh = mesh.borrow_mut::<HalfEdgeMesh>()?;
        crate::mesh::halfedge::edit_ops::make_group(&mut mesh, key_type, &selection, &group_name)
            .map_lua_err()?;
        Ok(())
    });

    lua_fn!(
        lua,
        ops,
        "set_material",
        |mesh: AnyUserData, selection: SelectionExpression, material_index: f32| -> () {
            let mut mesh = mesh.borrow_mut::<HalfEdgeMesh>()?;
            crate::mesh::halfedge::edit_ops::set_material(&mut mesh, &selection, material_index)
                .map_lua_err()?;
            Ok(())
        }
    );

    lua_fn!(
        lua,
        ops,
        "vertex_attribute_transfer",
        |src_mesh: AnyUserData,
         dst_mesh: AnyUserData,
         value_type: ChannelValueType,
         channel_name: String|
         -> () {
            use crate::mesh::halfedge::edit_ops::vertex_attribute_transfer;
            let src_mesh = src_mesh.borrow::<HalfEdgeMesh>()?;
            let mut dst_mesh = dst_mesh.borrow_mut::<HalfEdgeMesh>()?;
            match value_type {
                ChannelValueType::Vec3 => {
                    vertex_attribute_transfer::<glam::Vec3>(&src_mesh, &mut dst_mesh, &channel_name)
                }
                ChannelValueType::f32 => {
                    vertex_attribute_transfer::<f32>(&src_mesh, &mut dst_mesh, &channel_name)
                }
                ChannelValueType::bool => {
                    vertex_attribute_transfer::<bool>(&src_mesh, &mut dst_mesh, &channel_name)
                }
            }
            .map_lua_err()?;
            Ok(())
        }
    );

    lua_fn!(lua, ops, "set_full_range_uvs", |mesh: AnyUserData| -> () {
        let mut mesh = mesh.borrow_mut::<HalfEdgeMesh>()?;
        crate::mesh::halfedge::edit_ops::set_full_range_uvs(&mut mesh).map_lua_err()?;
        Ok(())
    });

    lua_fn!(lua, ops, "copy_to_points", |points: AnyUserData,
                                         mesh: AnyUserData|
     -> HalfEdgeMesh {
        let points = points.borrow::<HalfEdgeMesh>()?;
        let mesh = mesh.borrow::<HalfEdgeMesh>()?;
        crate::mesh::halfedge::edit_ops::copy_to_points(&points, &mesh).map_lua_err()
    });

    lua_fn!(
        lua,
        ops,
        "extrude_along_curve",
        |backbone: AnyUserData, cross_section: AnyUserData, flip: usize| -> HalfEdgeMesh {
            let backbone = backbone.borrow::<HalfEdgeMesh>()?;
            let cross_section = cross_section.borrow::<HalfEdgeMesh>()?;
            crate::mesh::halfedge::edit_ops::extrude_along_curve(&backbone, &cross_section, flip)
                .map_lua_err()
        }
    );

    crate::prelude::halfedge::edit_ops::lua_fns::__blackjack_register_lua_fns(lua);

    let types = lua.create_table()?;
    types.set("VertexId", ChannelKeyType::VertexId)?;
    types.set("FaceId", ChannelKeyType::FaceId)?;
    types.set("HalfEdgeId", ChannelKeyType::HalfEdgeId)?;
    types.set("Vec3", ChannelValueType::Vec3)?;
    types.set("f32", ChannelValueType::f32)?;
    types.set("bool", ChannelValueType::bool)?;
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

enum LuaTableKind {
    Sequential,
    Associative,
}

fn mesh_channel_to_lua_table<'lua>(
    lua: &'lua Lua,
    mesh: &HalfEdgeMesh,
    kty: ChannelKeyType,
    vty: ChannelValueType,
    ch_id: RawChannelId,
    kind: LuaTableKind,
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
    let ch = mesh
        .channels
        .dyn_read_channel(kty, vty, ch_id)
        .map_lua_err()?;

    match kind {
        LuaTableKind::Sequential => Ok(ch.to_seq_table(keys, lua)),
        LuaTableKind::Associative => Ok(ch.to_assoc_table(keys, lua)),
    }
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
                mesh_channel_to_lua_table(lua, this, kty, vty, ch_id, LuaTableKind::Sequential)
            },
        );
        methods.add_method(
            "get_assoc_channel",
            |lua, this, (kty, vty, name): (ChannelKeyType, ChannelValueType, String)| {
                let ch_id = this
                    .channels
                    .channel_id_dyn(kty, vty, &name)
                    .ok_or_else(|| anyhow::anyhow!("Channel '{name}' not found"))
                    .map_lua_err()?;
                mesh_channel_to_lua_table(lua, this, kty, vty, ch_id, LuaTableKind::Associative)
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
                .set_from_seq_table(keys, lua, table)
                .map_lua_err()
        });
        methods.add_method("set_assoc_channel", |lua, this, (kty, vty, name, table)| {
            let name: String = name;
            this.channels
                .dyn_write_channel_by_name(kty, vty, &name)
                .map_lua_err()?
                .set_from_assoc_table(lua, table)
                .map_lua_err()
        });
        methods.add_method_mut(
            "ensure_channel",
            |lua, this, (kty, vty, name): (ChannelKeyType, ChannelValueType, String)| {
                let id = this.channels.ensure_channel_dyn(kty, vty, &name);
                mesh_channel_to_lua_table(lua, this, kty, vty, id, LuaTableKind::Sequential)
            },
        );
        methods.add_method_mut(
            "ensure_assoc_channel",
            |lua, this, (kty, vty, name): (ChannelKeyType, ChannelValueType, String)| {
                let id = this.channels.ensure_channel_dyn(kty, vty, &name);
                mesh_channel_to_lua_table(lua, this, kty, vty, id, LuaTableKind::Associative)
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
            |_lua, this: &HalfEdgeMesh, v: VertexId| Ok(LVec3(this.read_positions()[v])),
        );
        methods.add_method_mut(
            "add_edge",
            |_lua, this: &mut HalfEdgeMesh, (start, end): (LVec3, LVec3)| {
                crate::prelude::halfedge::edit_ops::add_edge(this, start.0, end.0).map_lua_err()
            },
        );

        methods.add_method_mut("add_vertex", |_lua, this: &mut HalfEdgeMesh, pos: LVec3| {
            crate::prelude::halfedge::edit_ops::add_vertex(this, pos.0).map_lua_err()
        });

        methods.add_method(
            "halfedge_endpoints",
            |_lua, this: &HalfEdgeMesh, h: HalfEdgeId| -> mlua::Result<(LVec3, LVec3)> {
                let conn = this.read_connectivity();
                let positions = this.read_positions();
                let (src, dst) = conn.at_halfedge(h).src_dst_pair().map_lua_err()?;
                Ok((LVec3(positions[src]), LVec3(positions[dst])))
            },
        );

        methods.add_method(
            "halfedge_vertex_id",
            |_lua, this: &HalfEdgeMesh, h: HalfEdgeId| {
                this.read_connectivity()
                    .at_halfedge(h)
                    .vertex()
                    .try_end()
                    .map_lua_err()
            },
        );

        methods.add_method(
            "point_cloud",
            |_lua, this: &HalfEdgeMesh, sel: SelectionExpression| {
                crate::prelude::halfedge::edit_ops::point_cloud(this, sel).map_lua_err()
            },
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
