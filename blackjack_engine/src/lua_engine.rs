// Copyright (C) 2022 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{
    sync::{
        mpsc::{self, Receiver},
        Arc,
    },
    time::Duration,
};

use crate::{
    graph::NodeDefinitions, graph_compiler::ExternalParameterValues, mesh::heightmap::HeightMap,
    prelude::*,
};
use mlua::{Function, Lua};
use notify::{DebouncedEvent, Watcher};

use self::lua_stdlib::{load_node_definitions, LuaFileIo, StdLuaFileIo};

pub mod lua_stdlib;

pub trait ToLuaError<T> {
    fn map_lua_err(self) -> mlua::Result<T>;
}

impl<T> ToLuaError<T> for anyhow::Result<T> {
    fn map_lua_err(self) -> mlua::Result<T> {
        self.map_err(|err| mlua::Error::RuntimeError(format!("{:?}", err)))
    }
}

impl<T> ToLuaError<T> for Result<T, TraversalError> {
    fn map_lua_err(self) -> mlua::Result<T> {
        self.map_err(|err| mlua::Error::RuntimeError(format!("{:?}", err)))
    }
}

pub enum RenderableThing {
    HalfEdgeMesh(HalfEdgeMesh),
    HeightMap(HeightMap),
}

pub fn run_program<'lua>(
    lua: &'lua Lua,
    lua_program: &str,
    input: &ExternalParameterValues,
) -> Result<RenderableThing> {
    lua.load(lua_program).exec()?;
    let values = input.make_input_table(lua)?;
    let entry_point: Function = lua.globals().get("main")?;
    let result = entry_point
        .call::<_, mlua::AnyUserData>(values)
        .map_err(|err| anyhow!("{}", err))?;

    if result.is::<HalfEdgeMesh>() {
        Ok(RenderableThing::HalfEdgeMesh(result.take()?))
    } else if result.is::<HeightMap>() {
        Ok(RenderableThing::HeightMap(result.take()?))
    } else {
        Err(anyhow::anyhow!(
            "Object {result:?} is not a renderable thing"
        ))
    }
}

/// Like `run_program`, but does not return anything and only runs the code for
/// its side effects
pub fn run_program_side_effects<'lua>(
    lua: &'lua Lua,
    lua_program: &str,
    input: &ExternalParameterValues,
) -> Result<()> {
    lua.load(lua_program).exec()?;
    let values = input.make_input_table(lua)?;
    let entry_point: Function = lua.globals().get("main")?;
    entry_point
        .call::<_, mlua::Value>(values)
        .map_err(|err| anyhow!("{}", err))?;
    Ok(())
}

pub struct LuaRuntime {
    pub lua: Lua,
    pub node_definitions: NodeDefinitions,
    pub watcher: notify::RecommendedWatcher,
    pub watcher_channel: Receiver<notify::DebouncedEvent>,
    pub lua_io: Arc<dyn LuaFileIo + 'static>,
}

impl LuaRuntime {
    /// Initializes and returns the Blackjack Lua runtime. This function will
    /// use the `std::fs` API to load Lua source files. Some integrations may
    /// prefer to use other file reading methods with `initialize_custom`.
    pub fn initialize_with_std(node_libraries_path: String) -> anyhow::Result<LuaRuntime> {
        Self::initialize_custom(StdLuaFileIo {
            base_folder: node_libraries_path,
        })
    }

    pub fn initialize_custom(lua_io: impl LuaFileIo + 'static) -> anyhow::Result<LuaRuntime> {
        let lua = Lua::new();
        let lua_io = Arc::new(lua_io);
        lua_stdlib::load_host_libraries(&lua, lua_io.clone())?;
        lua_stdlib::load_lua_libraries(&lua)?;
        let node_definitions = load_node_definitions(&lua, lua_io.as_ref())?;
        let (watcher, watcher_channel) = {
            let (tx, rx) = mpsc::channel();
            let mut watcher = notify::watcher(tx, Duration::from_secs(1))?;
            watcher
                .watch(lua_io.base_folder(), notify::RecursiveMode::Recursive)
                .unwrap();
            (watcher, rx)
        };

        Ok(LuaRuntime {
            lua,
            node_definitions,
            watcher,
            watcher_channel,
            lua_io,
        })
    }

    pub fn watch_for_changes(&mut self) -> anyhow::Result<()> {
        if let Ok(msg) = self.watcher_channel.try_recv() {
            match msg {
                DebouncedEvent::Create(_)
                | DebouncedEvent::Write(_)
                | DebouncedEvent::Remove(_)
                | DebouncedEvent::Rename(_, _) => {
                    println!("Reloading Lua scripts...");
                    // Reset the _LOADED table to clear any required libraries
                    // from the cache. This will trigger reloading of libraries
                    // when the hot reloaded code first requires them,
                    // effectively picking up changes in transitively required
                    // libraries as well.
                    self.lua
                        .globals()
                        .set("_LOADED", self.lua.create_table()?)?;

                    // By calling this, all code under $BLACKJACK_LUA/run will
                    // be executed and the node definitions will be reloaded.
                    self.node_definitions = load_node_definitions(&self.lua, self.lua_io.as_ref())?;
                }
                _ => {}
            }
        }
        Ok(())
    }
}
