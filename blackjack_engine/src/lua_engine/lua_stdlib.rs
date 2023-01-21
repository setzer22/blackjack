// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::sync::Arc;

use mlua::{FromLua, Lua, Table, ToLua, UserData};

use crate::{
    lua_engine::ToLuaError,
    prelude::halfedge::{
        id_types::{FaceId, HalfEdgeId, VertexId},
        ChannelKeyType, ChannelValueType,
    },
};

mod runtime_types;
pub use runtime_types::*;

pub mod lua_require_io;
pub use lua_require_io::*;

mod lua_core_library;

pub mod lua_documentation;

/// A function pointer to register global lua functions. Stored globally using
/// the `inventory` crate.
pub struct LuaRegisterFn {
    pub f: fn(&mlua::Lua) -> mlua::Result<()>,
}
inventory::collect!(LuaRegisterFn);

/// Lua docstrings for symbol names. Stored globally using `inventory`.
pub struct LuaDocstringData {
    pub data: &'static [(&'static str, &'static str, &'static str)],
}
inventory::collect!(LuaDocstringData);

/// Loads all blackjack Rust function wrappers to the Lua API
pub fn load_lua_bindings(lua: &Lua, lua_io: Arc<dyn LuaFileIo + 'static>) -> anyhow::Result<()> {
    lua_core_library::load(lua, lua_io)?;

    // This collects functions from all over the codebase. Any module annotated
    // with `#[blackjack_macros::blackjack_lua_module]` is inspected and may
    // export any number of functions or constants marked with `#[lua]`
    // annotations.
    for register_fn in inventory::iter::<LuaRegisterFn>() {
        (register_fn.f)(lua).expect("Failed to register Lua API");
    }

    Ok(())
}
