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

use crate::{graph::NodeDefinitions, mesh::heightmap::HeightMap, prelude::*};
use mlua::Lua;
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

#[allow(clippy::large_enum_variant)]
pub enum RenderableThing {
    HalfEdgeMesh(HalfEdgeMesh),
    HeightMap(HeightMap),
}

impl RenderableThing {
    pub fn from_lua_value(renderable: mlua::Value<'_>) -> Result<Self> {
        match renderable {
            mlua::Value::UserData(renderable) if renderable.is::<HalfEdgeMesh>() => {
                Ok(RenderableThing::HalfEdgeMesh(renderable.take()?))
            }
            mlua::Value::UserData(renderable) if renderable.is::<HeightMap>() => {
                Ok(RenderableThing::HeightMap(renderable.take()?))
            }
            _ => {
                bail!("Object {renderable:?} is not a thing we can render.")
            }
        }
    }
}

/// The result of an invocation to a lua program.
pub struct ProgramResult {
    /// The renderable thing produced by this program to be shown on-screen.
    pub renderable: Option<RenderableThing>,
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
        lua_stdlib::load_lua_bindings(&lua, lua_io.clone())?;
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
