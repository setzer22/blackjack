use mlua::{AnyUserData, AsChunk, FromLua, Lua, Table, ToLua, UserData};

use crate::{
    lua_engine::ToLuaError,
    prelude::{
        compact_mesh::CompactMesh,
        graph::node_templates::{NodeDefinition, NodeDefinitions},
        halfedge::{
            id_types::{FaceId, HalfEdgeId, VertexId},
            ChannelKeyType, ChannelValueType, HalfEdgeMesh,
        },
        selection::SelectionExpression,
    },
};

/// Convenience macro for registering a lua function inside a global table.
macro_rules! lua_fn {
    ($lua:ident, $table:ident, $name:expr, || -> $retval:ty { $($body:tt)* }) => {
        $table.set($name,
            #[allow(unused_parens)]
            #[allow(unused_variables)]
            $lua.create_function(|$lua, ()| -> mlua::Result<$retval> {
                $($body)*
            })?
        )?
    };
    ($lua:ident, $table:ident, $name:expr, |$($argname:ident : $typ:ty),*| -> $retval:ty { $($body:tt)* }) => {
        $table.set($name,
            #[allow(unused_parens)]
            #[allow(unused_variables)]
            $lua.create_function(|$lua, ($($argname),*) : ($($typ),*)| -> mlua::Result<$retval> {
                $($body)*
            })?
        )?
    };
}

mod runtime_types;
pub use runtime_types::*;
mod lua_constructors_library;
mod lua_export_library;
mod lua_mesh_library;
mod lua_node_libraries;
mod lua_primitives_library;

/// Loads pure Lua libraries that are part of the blackjack core APIs
pub fn load_lua_libraries(lua: &Lua) -> anyhow::Result<()> {
    macro_rules! def_library {
        ($name:expr, $file:expr) => {
            let lib: mlua::Value = lua.load(include_str!($file)).call(())?;
            lua.globals().set($name, lib)?;
        };
    }

    def_library!("NodeLibrary", "node_library.lua");
    Ok(())
}

/// Loads all blackjack Rust function wrappers to the Lua API
pub fn load_host_libraries(lua: &Lua) -> anyhow::Result<()> {
    lua_mesh_library::load(lua)?;
    lua_primitives_library::load(lua)?;
    lua_export_library::load(lua)?;
    lua_constructors_library::load(lua)?;
    Ok(())
}

/// Loads all the registered node libraries by running all the files in the
/// NODE_LIBRARIES_PATH folder and returns the node definitions for the
/// registered nodes found.
pub fn load_node_libraries(lua: &Lua) -> anyhow::Result<NodeDefinitions> {
    lua_node_libraries::load(lua)
}
