use crate::prelude::*;
use mlua::{Function, Lua, Table};

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
    lua.load(&lua_program).exec()?;
    let entry_point: Function = lua.globals().get("main")?;
    let mesh = entry_point
        .call::<_, HalfEdgeMesh>(input)
        .map_err(|err| anyhow!("{}", err))?;
    Ok(mesh)
}
