// Copyright (C) 2022 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use anyhow::Result;
use gdnative::prelude::*;

use blackjack_engine::lua_engine::lua_stdlib::{LuaFileIo, LuaSourceFile};

use gdnative::api as gd;

pub struct GodotLuaIo {
    pub base_folder: String,
}

impl LuaFileIo for GodotLuaIo {
    fn base_folder(&self) -> &str {
        &self.base_folder
    }

    fn find_run_files(&self) -> Box<dyn Iterator<Item = String>> {
        pub fn find_files_recursive(
            path: GodotString,
            files: &mut Vec<String>,
        ) -> anyhow::Result<()> {
            let folder = gd::Directory::new();
            folder.open(path)?;
            folder.list_dir_begin(true, false)?;
            let mut file_name = folder.get_next();
            while file_name != "".into() {
                if folder.current_is_dir() {
                    find_files_recursive(folder.get_current_dir(), files)?;
                } else if file_name.ends_with(&GodotString::from_str(".lua")) {
                    let path =
                        folder.get_current_dir() + GodotString::from_str("/") + file_name.clone();
                    files.push(path.to_string());
                }

                file_name = folder.get_next();
            }
            Ok(())
        }
        let mut files = vec![];
        match find_files_recursive(
            GodotString::from(self.base_folder.clone() + "/run"),
            &mut files,
        ) {
            Ok(_) => Box::new(files.into_iter()),
            Err(err) => {
                godot_error!("There was an error when loading blackjack files: {err}");
                Box::new(std::iter::empty())
            }
        }
    }

    fn load_file_absolute(
        &self,
        path: &str,
    ) -> anyhow::Result<blackjack_engine::lua_engine::lua_stdlib::LuaSourceFile> {
        let file = gd::File::new();
        file.open(path, gd::File::READ)?;
        let contents = file.get_as_text();
        Ok(LuaSourceFile {
            contents: contents.to_string(),
            name: path.to_string(),
        })
    }

    fn load_file_require(
        &self,
        path: &str,
    ) -> anyhow::Result<blackjack_engine::lua_engine::lua_stdlib::LuaSourceFile> {
        let mut path = String::from(self.base_folder.clone() + "/lib/" + path);
        if !path.ends_with(".lua") {
            path += ".lua";
        }
        self.load_file_absolute(&path)
    }
}
