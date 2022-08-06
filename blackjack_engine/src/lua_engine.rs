// Copyright (C) 2022 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{
    sync::mpsc::{self, Receiver},
    time::Duration,
};

use crate::{
    graph::NodeDefinitions, graph_compiler::ExternalParameterValues, mesh::heightmap::HeightMap,
    prelude::*,
};
use mlua::{FromLua, Function, Lua};
use notify::{DebouncedEvent, Watcher};

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

pub struct LuaRuntime {
    pub lua: Lua,
    pub node_definitions: NodeDefinitions,
    pub watcher: notify::RecommendedWatcher,
    pub watcher_channel: Receiver<notify::DebouncedEvent>,
    pub node_libraries_path: String,
    pub load_libraries_fn: Box<dyn Fn(&Lua, &str) -> Result<NodeDefinitions>>,
}

const NODE_LIBRARIES_PATH: &str = "node_libraries";

impl LuaRuntime {
    /// Initializes and returns the Blackjack Lua runtime. This function will
    /// use the `std::fs` API to load Lua source files. Some integrations may
    /// prefer to use other file reading methods with `initialize_custom`.
    pub fn initialize_with_std(node_libraries_path: String) -> anyhow::Result<LuaRuntime> {
        Self::initialize_custom(
            node_libraries_path,
            lua_stdlib::load_node_libraries_with_std,
        )
    }

    pub fn initialize_custom(
        node_libraries_path: String,
        load_libraries_fn: impl Fn(&Lua, &str) -> Result<NodeDefinitions> + 'static,
    ) -> anyhow::Result<LuaRuntime> {
        let lua = Lua::new();
        lua_stdlib::load_host_libraries(&lua)?;
        lua_stdlib::load_lua_libraries(&lua)?;
        let node_definitions = load_libraries_fn(&lua, &node_libraries_path)?;
        let (watcher, watcher_channel) = {
            let (tx, rx) = mpsc::channel();
            let mut watcher = notify::watcher(tx, Duration::from_secs(1))?;
            watcher
                .watch(NODE_LIBRARIES_PATH, notify::RecursiveMode::Recursive)
                .unwrap();
            (watcher, rx)
        };

        Ok(LuaRuntime {
            lua,
            node_definitions,
            watcher,
            watcher_channel,
            node_libraries_path,
            load_libraries_fn: Box::new(load_libraries_fn),
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
                    self.node_definitions =
                        (self.load_libraries_fn)(&self.lua, &self.node_libraries_path)?;
                }
                _ => {}
            }
        }
        Ok(())
    }
}
