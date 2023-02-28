// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::*;
use crate::{
    lua_engine::{lua_stdlib::LVec3, ToLuaError},
    sync::RefCounted,
};
use mlua::{Function, Lua, ToLua, Value};

#[blackjack_macros::blackjack_lua_module]
#[allow(non_upper_case_globals)]
mod lua_api {
    use mlua::{Function, Table, Value};

    use super::{selection::SelectionExpression, *};

    #[lua(under = "HalfEdgeMesh")]
    fn new() -> HalfEdgeMesh {
        HalfEdgeMesh::default()
    }

    /// The 'vertex' is one of the three mesh elements. Channels attached to
    /// vertices have this key type.
    #[lua(under = "Types")]
    const VERTEX_ID: ChannelKeyType = ChannelKeyType::VertexId;

    /// The 'face' is one of the three mesh elements. Channels attached to faces
    /// have this key type.
    #[lua(under = "Types")]
    const FACE_ID: ChannelKeyType = ChannelKeyType::FaceId;

    /// The 'face' is one of the three mesh elements. Channels attached to faces
    /// have this key type.
    #[lua(under = "Types")]
    const HALFEDGE_ID: ChannelKeyType = ChannelKeyType::HalfEdgeId;

    /// The type of vector channels associated to a mesh element.
    #[lua(under = "Types")]
    const VEC3: ChannelValueType = ChannelValueType::Vec3;

    /// The type of scalar channels associated to a mesh element.
    #[lua(under = "Types")]
    const F32: ChannelValueType = ChannelValueType::f32;

    /// The type of boolean channels (groups) associated to a mesh element.
    #[lua(under = "Types")]
    const BOOL: ChannelValueType = ChannelValueType::bool;

    #[lua_impl]
    impl HalfEdgeMesh {
        // ==== CORE ====

        /// Duplicates this mesh by deep-cloning all its data.
        #[lua(hidden)]
        fn clone(&self) -> HalfEdgeMesh {
            self.clone()
        }

        // ==== CHANNEL MANAGEMENT ====

        /// Returns a mesh channel with key type `kty`, value type `vty` and
        /// `name` as a sequential table. The values for the channel will all be
        /// returned in the same iteration order, so this mode is ideal for
        /// parallel iteration of data from multiple channels, where knowing the
        /// exact vertex / face / halfedge id is not important.
        #[lua(hidden)]
        fn get_channel<'lua>(
            &self,
            lua: &'lua Lua,
            kty: ChannelKeyType,
            vty: ChannelValueType,
            name: String,
        ) -> Result<mlua::Table<'lua>> {
            let ch_id = self
                .channels
                .channel_id_dyn(kty, vty, &name)
                .ok_or_else(|| anyhow::anyhow!("Channel '{name}' not found"))?;
            mesh_channel_to_lua_table(lua, self, kty, vty, ch_id, LuaTableKind::Sequential)
        }

        /// Returns a mesh channel with key type `kty`, value type `vty` and
        /// `name` as an associative table, where keys are ids (either VertexId,
        /// FaceId or HalfEdgeId). This mode is less efficient than sequential,
        /// but allows for more complex manipulation since you can iterate the
        /// data alongside the ids.
        #[lua(hidden)]
        fn get_assoc_channel<'lua>(
            &self,
            lua: &'lua Lua,
            kty: ChannelKeyType,
            vty: ChannelValueType,
            name: String,
        ) -> Result<mlua::Table<'lua>> {
            let ch_id = self
                .channels
                .channel_id_dyn(kty, vty, &name)
                .ok_or_else(|| anyhow::anyhow!("Channel '{name}' not found"))?;
            mesh_channel_to_lua_table(lua, self, kty, vty, ch_id, LuaTableKind::Associative)
        }

        /// Returns a mesh channel with key type `kty`, value type `vty` and
        /// `name` as a shared channel. Unlike sequential and associative
        /// channels, a shared channel does not have its data copied to Lua.
        /// This can be much more efficient if you only plan to access a small
        /// subset of elements, but can be an order of magnitude slower if every
        /// element needs to be accessed.
        ///
        /// The shared channel can be used like a regular Lua table, using the
        /// index operator to query or set keys, using the right channel keys as
        /// values.
        #[lua(hidden)]
        fn get_shared_channel(
            &self,
            kty: ChannelKeyType,
            vty: ChannelValueType,
            name: String,
        ) -> Result<SharedChannel> {
            self.channels
                .channel_rc_dyn(kty, vty, &name)
                .map(|x| SharedChannel(x))
        }

        /// Sets a mesh channel with key type `kty`, value type `vty` and `name`
        /// from a sequential table. The table should be a sequence of values,
        /// one for each element (vertex, halfedge, face) equivalent to the one
        /// that would be obtained via `HalfEdgeMesh::get_channel`.
        #[lua(hidden)]
        fn set_channel(
            &self,
            lua: &Lua,
            kty: ChannelKeyType,
            vty: ChannelValueType,
            name: String,
            table: Table,
        ) -> Result<()> {
            use slotmap::Key;
            let name: String = name;
            let conn = self.read_connectivity();
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
            self.channels
                .dyn_write_channel_by_name(kty, vty, &name)?
                .set_from_seq_table(keys, lua, table)
        }

        /// Sets a mesh channel with key type `kty`, value type `vty` and `name`
        /// from an associative table. The table should be a mapping of ids to
        /// values, one for each element (vertex, halfedge, face) equivalent to
        /// the one obtained via `HalfEdgeMesh::get_assoc_channel`.
        #[lua(hidden)]
        fn set_assoc_channel(
            &self,
            lua: &Lua,
            kty: ChannelKeyType,
            vty: ChannelValueType,
            name: String,
            table: Table,
        ) -> Result<()> {
            let name: String = name;
            self.channels
                .dyn_write_channel_by_name(kty, vty, &name)?
                .set_from_assoc_table(lua, table)
        }

        /// Same as `HalfEdgeMesh::get_channel`, but creates the channel if one
        /// didn't exist already.
        #[lua(hidden)]
        fn ensure_channel<'lua>(
            &mut self,
            lua: &'lua Lua,
            kty: ChannelKeyType,
            vty: ChannelValueType,
            name: String,
        ) -> Result<Table<'lua>> {
            let id = self.channels.ensure_channel_dyn(kty, vty, &name);
            mesh_channel_to_lua_table(lua, self, kty, vty, id, LuaTableKind::Sequential)
        }

        /// Same as `HalfEdgeMesh::get_assoc_channel`, but creates the channel
        /// if one didn't exist already.
        #[lua(hidden)]
        fn ensure_assoc_channel<'lua>(
            &mut self,
            lua: &'lua Lua,
            kty: ChannelKeyType,
            vty: ChannelValueType,
            name: String,
        ) -> Result<Table<'lua>> {
            let id = self.channels.ensure_channel_dyn(kty, vty, &name);
            mesh_channel_to_lua_table(lua, self, kty, vty, id, LuaTableKind::Associative)
        }

        // ==== ITERATION ====

        /// Returns an iterator over the vertices of this mesh. Vertex ids are
        /// not useful on their own, but can be used to retrieve data by calling
        /// other methods.
        #[lua(hidden)]
        fn iter_vertices<'lua>(&self, lua: &'lua Lua) -> mlua::Result<Function<'lua>> {
            let vertices: Vec<VertexId> = self
                .read_connectivity()
                .iter_vertices()
                .map(|(id, _)| id)
                .collect();
            let mut i = 0;
            lua.create_function_mut(move |lua, ()| {
                let val = if i < vertices.len() {
                    vertices[i].to_lua(lua)?
                } else {
                    mlua::Value::Nil
                };
                i += 1;
                Ok(val)
            })
        }

        /// Returns an iterator over the halfedges of this mesh. HalfEdge ids
        /// are not useful on their own, but can be used to retrieve data by
        /// calling other methods.
        #[lua(hidden)]
        fn iter_halfedges<'lua>(&self, lua: &'lua Lua) -> mlua::Result<Function<'lua>> {
            let halfedges: Vec<HalfEdgeId> = self
                .read_connectivity()
                .iter_halfedges()
                .map(|(id, _)| id)
                .collect();
            let mut i = 0;
            lua.create_function_mut(move |lua, ()| {
                let val = if i < halfedges.len() {
                    halfedges[i].to_lua(lua)?
                } else {
                    mlua::Value::Nil
                };
                i += 1;
                Ok(val)
            })
        }

        /// Returns an iterator over the faces of this mesh. Face ids are not
        /// useful on their own, but can be used to retrieve data by calling
        /// other methods.
        #[lua(hidden)]
        fn iter_faces<'lua>(&self, lua: &'lua Lua) -> mlua::Result<Function<'lua>> {
            let halfedges: Vec<FaceId> = self
                .read_connectivity()
                .iter_faces()
                .map(|(id, _)| id)
                .collect();
            let mut i = 0;
            lua.create_function_mut(move |lua, ()| {
                let val = if i < halfedges.len() {
                    halfedges[i].to_lua(lua)?
                } else {
                    mlua::Value::Nil
                };
                i += 1;
                Ok(val)
            })
        }

        // ==== REDUCTIONS ====

        /// A reduction over an element type of this mesh (vertex, face,
        /// halfedge). Returns the result of calling f with `init` and the first
        /// mesh element, then feed the result into `f` again with the second
        /// mesh element, and so on for all elements of the mesh.
        ///
        /// This is similar to the `reduce` function in other languages:
        /// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Array/reduce
        #[lua(hidden)]
        fn reduce<'lua>(
            &self,
            kty: ChannelKeyType,
            init: Value<'lua>,
            f: Function<'lua>,
        ) -> mlua::Result<Value<'lua>> {
            mesh_reduce(self, kty, init, f)
        }

        /// Same as `HalfEdgeMesh::reduce`, but specifically for `VertexId`
        #[lua(hidden)]
        fn reduce_vertices<'lua>(
            &self,
            init: Value<'lua>,
            f: Function<'lua>,
        ) -> mlua::Result<Value<'lua>> {
            mesh_reduce(self, ChannelKeyType::VertexId, init, f)
        }

        /// Same as `HalfEdgeMesh::reduce`, but specifically for `HalfEdgeId`
        #[lua(hidden)]
        fn reduce_halfedges<'lua>(
            &self,
            init: Value<'lua>,
            f: Function<'lua>,
        ) -> mlua::Result<Value<'lua>> {
            mesh_reduce(self, ChannelKeyType::HalfEdgeId, init, f)
        }

        /// Same as `HalfEdgeMesh::reduce`, but specifically for `FaceId`
        #[lua(hidden)]
        fn reduce_faces<'lua>(
            &self,
            init: Value<'lua>,
            f: Function<'lua>,
        ) -> mlua::Result<Value<'lua>> {
            mesh_reduce(self, ChannelKeyType::FaceId, init, f)
        }

        // ==== VERTEX GETTERS ====

        /// Returns the position of a vertex with `vertex_id`.
        #[lua]
        pub fn vertex_position(&self, vertex_id: VertexId) -> LVec3 {
            self.read_positions()[vertex_id].into()
        }

        // ==== FACE GETTERS ====

        /// Returns a list of edges for the given `face_id`.
        #[lua(this = "read_connectivity()", map = "x.to_vec()")]
        pub fn face_edges(&self, face_id: FaceId) -> Vec<HalfEdgeId>;

        #[lua]
        pub fn face_vertices(&self, face_id: FaceId) -> Result<Vec<VertexId>> {
            Ok(self
                .read_connectivity()
                .at_face(face_id)
                .vertices()?
                .to_vec())
        }

        /// Given a `SelectionExpression`, returns all halfedge ids in this mesh
        /// matching it.
        #[lua]
        pub fn resolve_halfedge_selection_full(
            &self,
            selection: &SelectionExpression,
        ) -> Result<Vec<HalfEdgeId>>;

        /// Given a `SelectionExpression`, returns all vertex ids in this mesh
        /// matching it.
        #[lua]
        pub fn resolve_vertex_selection_full(
            &self,
            selection: &SelectionExpression,
        ) -> Result<Vec<VertexId>>;

        /// Given a `SelectionExpression`, returns all face ids in this mesh
        /// matching it.
        #[lua]
        pub fn resolve_face_selection_full(
            &self,
            selection: &SelectionExpression,
        ) -> Result<Vec<FaceId>>;

        // ==== HALFEDGE GETTERS ====

        /// Returns the endpoint positions of the given halfedge `h`.
        #[lua]
        pub fn halfedge_endpoints(&self, h: HalfEdgeId) -> Result<(LVec3, LVec3)> {
            let conn = self.read_connectivity();
            let positions = self.read_positions();
            let (src, dst) = conn.at_halfedge(h).src_dst_pair()?;
            Ok((LVec3(positions[src]), LVec3(positions[dst])))
        }

        #[lua]
        pub fn halfedge_vertex_id(&self, h: HalfEdgeId) -> Result<VertexId> {
            Ok(self.read_connectivity().at_halfedge(h).vertex().try_end()?)
        }

        #[lua]
        pub fn halfedge_vertices(&self, halfedge_id: HalfEdgeId) -> Result<(VertexId, VertexId)> {
            Ok(self
                .read_connectivity()
                .at_halfedge(halfedge_id)
                .src_dst_pair()?)
        }

        // ==== OPS ====

        /// Adds a new disconnected edge to this mesh with endpoints `start` and
        /// `end`.
        #[lua]
        pub fn add_edge(&mut self, start: LVec3, end: LVec3) -> Result<(HalfEdgeId, HalfEdgeId)> {
            crate::prelude::halfedge::edit_ops::add_edge(self, start.0, end.0)
        }

        /// Adds an empty vertex to the mesh at `pos`. Useful when the mesh is
        /// representing a point cloud. Otherwise it's preferrable to use
        /// higher-level operators
        #[lua]
        pub fn add_vertex(&mut self, pos: LVec3) -> Result<()> {
            crate::prelude::halfedge::edit_ops::add_vertex(self, pos.0)
        }

        /// Returns a point cloud mesh, selecting a set of vertices `sel` from
        /// this mesh.
        #[lua]
        pub fn point_cloud(&self, sel: SelectionExpression) -> Result<HalfEdgeMesh> {
            crate::prelude::halfedge::edit_ops::point_cloud(self, sel)
        }
    }
}
pub use lua_api::*;

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
) -> anyhow::Result<mlua::Table<'lua>> {
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
    let ch = mesh.channels.dyn_read_channel(kty, vty, ch_id)?;

    match kind {
        LuaTableKind::Sequential => Ok(ch.to_seq_table(keys, lua)),
        LuaTableKind::Associative => Ok(ch.to_assoc_table(keys, lua)),
    }
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

pub struct SharedChannel(pub RefCounted<InteriorMutable<dyn DynChannel>>);
impl Clone for SharedChannel {
    fn clone(&self) -> Self {
        Self(RefCounted::clone(&self.0))
    }
}

impl mlua::UserData for SharedChannel {
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
            Ok(value)
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
