use super::*;
use mlua::Lua;

#[blackjack_macros::blackjack_lua_module]
mod lua_api {
    use mlua::Table;

    use super::*;

    #[lua(under = "HalfEdgeMesh")]
    fn new() -> HalfEdgeMesh {
        HalfEdgeMesh::default()
    }

    #[lua_impl]
    impl HalfEdgeMesh {
        /// Returns a list of edges for the given `face_id`.
        #[lua(this = "read_connectivity()", map = "x.to_vec()")]
        pub fn face_edges(&self, face_id: FaceId) -> Vec<HalfEdgeId>;

        /// Returns a mesh channel with key type `kty` and value type `vty` and
        /// `name` as a sequential table. The values for the channel will all be
        /// returned in the same iteration order, so this mode is ideal for
        /// parallel iteration of data from multiple channels, where knowing the
        /// exact vertex / face / halfedge id is not important.
        #[lua(hidden)]
        pub fn get_channel<'lua>(
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

        /// Returns a mesh channel with key type `kty` and value type `vty` and
        /// `name` as an associative table, where keys are ids (either VertexId,
        /// FaceId or HalfEdgeId). This mode is less efficient than sequential,
        /// but allows for more complex manipulation since you can iterate the
        /// data alongside the ids.
        #[lua(hidden)]
        pub fn get_assoc_channel<'lua>(
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

        #[lua(hidden)]
        pub fn set_channel<'lua>(
            &self,
            lua: &'lua Lua,
            kty: ChannelKeyType,
            vty: ChannelValueType,
            name: String,
            table: Table,
        ) {
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
            // WIP: I'm porting functions over the commented block from
            // lua_mesh_library. This fails to compile...
            self.channels
                .dyn_write_channel_by_name(kty, vty, &name)?
                .set_from_seq_table(keys, lua, table);
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
