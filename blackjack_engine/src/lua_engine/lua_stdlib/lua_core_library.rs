// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

// Copyright (C) 2022 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::sync::Arc;

use mlua::Value;

use super::*;

pub fn load(lua: &Lua, lua_io: Arc<dyn LuaFileIo + 'static>) -> anyhow::Result<()> {
    let globals = lua.globals();

    // The _LOADED table stores things loaded by the `require` function
    globals.set("_LOADED", lua.create_table()?)?;

    globals.set(
        "require",
        lua.create_function(move |lua, file: String| -> Result<mlua::Value, _> {
            let loaded: Table = lua
                .globals()
                .get("_LOADED")
                .expect("The _LOADED table must always exist");
            match loaded.get::<_, Value>(file.clone())? {
                Value::Nil => {
                    macro_rules! def_lib {
                        ($req_name:expr, $path:expr) => {
                            if file == $req_name {
                                let value = lua.load(include_str!($path)).eval::<mlua::Value>()?;
                                loaded.set(file, value.clone())?;
                                return Ok(value);
                            }
                        };
                    }

                    // Here we list all the predefined libraries. These are
                    // baked into the blackjack binary via `include_str!`
                    def_lib!("params", "../node_params.lua");
                    def_lib!("node_library", "../node_library.lua");
                    def_lib!("utils", "../blackjack_utils.lua");

                    // The `def_lib!` calls above return. If none did, then we
                    // know this is a regular lua file from the filesystem.
                    {
                        let file_chunk = lua_io.load_file_require(&file).map_lua_err()?;
                        let value = lua.load(&file_chunk).eval::<mlua::Value>()?;
                        loaded.set(file, value.clone())?;
                        Ok(value)
                    }
                }
                other => Ok(other),
            }
        })?,
    )?;

    globals.set(
        "loadstring",
        lua.create_function(|lua, s: String| -> Result<mlua::MultiValue, _> {
            match lua.load(&s).eval::<mlua::Value>() {
                Ok(v) => Ok(mlua::MultiValue::from_vec(vec![v])),
                Err(err) => Ok(mlua::MultiValue::from_vec(vec![
                    mlua::Nil,
                    format!("{err}").to_lua(lua)?,
                ])),
            }
        })?,
    )?;

    Ok(())
}

#[blackjack_macros::blackjack_lua_module]
mod lua_module {
    use anyhow::Result;

    /// Read the contents of the file at `path` and return as a string. Will
    /// fail if the path does not exist, or the user does not have correct
    /// access permissions.
    #[lua(under = "Io")]
    pub fn read_to_string(path: String) -> Result<String> {
        Ok(std::fs::read_to_string(path)?)
    }

    /// Write the given string `contents` as a file to the given `path`. Will
    /// overwrite any previous existing file with that name.
    #[lua(under = "Io")]
    pub fn write(path: String, contents: String) -> Result<()> {
        std::fs::write(path, contents)?;
        Ok(())
    }
}
