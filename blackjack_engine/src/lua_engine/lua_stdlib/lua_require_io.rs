// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{borrow::Cow, path::PathBuf};

use crate::graph::{NodeDefinition, NodeDefinitionsInner};

pub struct LuaSourceFile {
    pub contents: String,
    pub name: String,
}

impl<'lua> mlua::AsChunk<'lua> for LuaSourceFile {
    fn source(&self) -> std::result::Result<Cow<'_, [u8]>, std::io::Error> {
        Ok(Cow::Borrowed(self.contents.as_bytes()))
    }

    fn name(&self) -> std::option::Option<std::string::String> {
        Some(self.name.clone())
    }
}

/// This trait is used to abstract Lua file IO to allow different
/// implementations for different platforms. Some game engines, like Godot, use
/// packed binary data and offer their own `File` APIs to handle that
/// abstraction, using regular std::io in those cases would not work.
///
/// If you are writing an integration and your engine can load assets using
/// Rust's standard io functions, you can use the `StdLuaFileIo` default
/// implementation which uses the Rust standard library.
pub trait LuaFileIo {
    /// Returns the path of the base folder this FileIo is watching. This is
    /// used in blackjack_ui for hot reloading. Other integrations may use it in
    /// different ways.
    fn base_folder(&self) -> &str;

    /// Returns an iterator over the paths of all the blackjack initialization
    /// scripts on the lua folder.
    ///
    /// The calling code does not care about the format of the returned paths.
    /// The values should be valid to call the `load_file_absolute` function,
    /// which in practice means they should be absolute paths.
    fn find_run_files(&self) -> Box<dyn Iterator<Item = String>>;

    /// Returns a [`LuaSourceFile`] with the contents of the file at a given
    /// `path`. The path will be treated as absolute.
    fn load_file_absolute(&self, path: &str) -> anyhow::Result<LuaSourceFile>;

    /// Returns a [`LuaSourceFile`] with the contents of the file at a given
    /// `path`. The path is relative to $BLACKJACK_LUA/lib. This function will
    /// be used when Lua code calls the `require` function.
    fn load_file_require(&self, path: &str) -> anyhow::Result<LuaSourceFile>;
}

pub struct StdLuaFileIo {
    pub base_folder: String,
}

impl LuaFileIo for StdLuaFileIo {
    fn base_folder(&self) -> &str {
        &self.base_folder
    }

    fn find_run_files(&self) -> Box<dyn Iterator<Item = String>> {
        let run_path = PathBuf::from(&self.base_folder).join("run");
        Box::new(
            walkdir::WalkDir::new(run_path)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_type().is_file()
                        && e.file_name()
                            .to_str()
                            .map(|s| s.ends_with(".lua"))
                            .unwrap_or(false)
                })
                .filter_map(|e| e.path().to_str().map(|x| x.to_owned())),
        )
    }

    fn load_file_absolute(&self, path: &str) -> anyhow::Result<LuaSourceFile> {
        Ok(LuaSourceFile {
            contents: std::fs::read_to_string(path)?,
            name: path.into(),
        })
    }

    fn load_file_require(&self, path: &str) -> anyhow::Result<LuaSourceFile> {
        let mut path = PathBuf::from(&self.base_folder).join("lib").join(path);
        path.set_extension("lua");
        Ok(LuaSourceFile {
            contents: std::fs::read_to_string(&path).map_err(|err| {
                anyhow::anyhow!("Error loading file {}. Cause: {err}", path.display())
            })?,
            name: path.display().to_string(),
        })
    }
}

/// Scans and runs all files inside $BLACKJACK_LUA/run. Then, parses every
/// registered node and returns a `NodeDefinitions` object with the nodes.
pub fn load_node_definitions(
    lua: &mlua::Lua,
    lua_io: &dyn LuaFileIo,
) -> anyhow::Result<NodeDefinitionsInner> {
    for path in lua_io.find_run_files() {
        let file = lua_io.load_file_absolute(&path)?;
        lua.load(&file).exec()?;
    }

    let table = lua
        .load("require('node_library')")
        .eval::<mlua::Table>()?
        .get::<_, mlua::Table>("nodes")?;
    NodeDefinition::load_nodes_from_table(table)
}
