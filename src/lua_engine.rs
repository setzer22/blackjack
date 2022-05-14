use std::{
    sync::mpsc::{self, Receiver},
    time::Duration,
};

use crate::prelude::{graph::node_templates::NodeDefinitions, *};
use mlua::{Function, Lua, Table};
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

pub fn run_program<'lua>(
    lua: &'lua Lua,
    lua_program: &str,
    input: Table<'lua>,
) -> Result<HalfEdgeMesh> {
    lua.load(lua_program).exec()?;
    let entry_point: Function = lua.globals().get("main")?;
    let mesh = entry_point
        .call::<_, HalfEdgeMesh>(input)
        .map_err(|err| anyhow!("{}", err))?;
    Ok(mesh)
}

pub struct LuaRuntime {
    pub lua: Lua,
    pub node_definitions: NodeDefinitions,
    pub watcher: notify::RecommendedWatcher,
    pub watcher_channel: Receiver<notify::DebouncedEvent>,
}

const NODE_LIBRARIES_PATH: &str = "node_libraries";

impl LuaRuntime {
    pub fn initialize() -> anyhow::Result<LuaRuntime> {
        let lua = Lua::new();
        lua_stdlib::load_host_libraries(&lua)?;
        lua_stdlib::load_lua_libraries(&lua)?;
        let node_definitions = lua_stdlib::load_node_libraries(&lua)?;
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
                    self.node_definitions = lua_stdlib::load_node_libraries(&self.lua)?;
                }
                _ => {}
            }
        }
        Ok(())
    }
}
