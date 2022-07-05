// Copyright (C) 2022 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use anyhow::Result;
use gdnative::prelude::*;
use std::borrow::Cow;

use blackjack_engine::graph::{NodeDefinition, NodeDefinitions};
use mlua::{AsChunk, Lua, Table};

use gdnative::api as gd;

pub struct LuaSourceFile {
    contents: String,
    name: String,
}

impl<'lua> AsChunk<'lua> for LuaSourceFile {
    fn source(&self) -> std::result::Result<Cow<'_, [u8]>, std::io::Error> {
        Ok(Cow::Borrowed(self.contents.as_bytes()))
    }

    fn name(&self) -> std::option::Option<std::string::String> {
        Some(self.name.clone())
    }
}

pub fn load_node_libraries_with_godot(
    lua: &Lua,
    node_libs_path: &str,
) -> anyhow::Result<NodeDefinitions> {
    pub fn eval_recursive(lua: &Lua, path: GodotString) -> Result<()> {
        let folder = gd::Directory::new();
        folder.open(path)?;
        folder.list_dir_begin(true, false)?;
        let mut file_name = folder.get_next();
        while file_name != "".into() {
            if folder.current_is_dir() {
                eval_recursive(lua, folder.get_current_dir())?;
            } else if file_name.ends_with(&GodotString::from_str(".lua")) {
                let path =
                    folder.get_current_dir() + GodotString::from_str("/") + file_name.clone();
                let file = gd::File::new();
                file.open(path.clone(), gd::File::READ)?;
                let contents = file.get_as_text().to_string();
                lua.load(&LuaSourceFile {
                    contents,
                    name: path.to_string(),
                })
                .exec()?;
            }

            file_name = folder.get_next();
        }
        Ok(())
    }

    eval_recursive(lua, node_libs_path.into())?;

    /*for entry in walkdir::WalkDir::new(node_libs_path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let is_lua_file = entry.file_type().is_file()
            && entry
                .file_name()
                .to_str()
                .map(|s| s.ends_with(".lua"))
                .unwrap_or(false);

        if is_lua_file {
            let path = entry.path();

            let path_display = format!("{}", path.display());

            println!("Loading Lua file {}", path_display);

            lua.load(&LuaSourceFile {
                contents: std::fs::read_to_string(path).unwrap_or_else(|err| {
                    format!("error('Error reading file \"{:?}\". {}')", path, err)
                }),
                name: path_display,
            })
            .exec()?;
        }
    }*/

    let table = lua
        .globals()
        .get::<_, Table>("NodeLibrary")?
        .get::<_, Table>("nodes")?;
    NodeDefinition::load_nodes_from_table(table)
}
