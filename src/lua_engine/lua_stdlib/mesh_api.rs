use super::*;
use mlua::UserData;

use crate::prelude::*;

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
                    .ok_or_else(|| anyhow!("Channel '{name}' not found"))
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
        methods.add_method_mut("iter_vertices", |lua, this, ()| {
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
                    mlua::Value::Nil
                };
                i += 1;
                Ok(val)
            })
        });
        methods.add_method("clone", |_lua, this, ()| Ok(this.clone()));
    }
}
