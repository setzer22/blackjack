// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use anyhow::{bail, Result};
use std::io::Write;
use std::{collections::BTreeMap, fs::File, path::PathBuf};

use crate::lua_engine::lua_stdlib::LuaDocstringData;

pub fn generate_lua_documentation(out_path: &str) -> Result<()> {
    let mut docs_by_module = BTreeMap::<&str, Vec<&str>>::new();
    let mut docs_by_class = BTreeMap::<&str, Vec<&str>>::new();

    fn populate_from<'a>(
        it: &'a [(&'a str, &'a str, &'a str)],
        docs_by_module: &mut BTreeMap<&'a str, Vec<&'a str>>,
        docs_by_class: &mut BTreeMap<&'a str, Vec<&'a str>>,
    ) {
        for (t, m, f) in it {
            if *t == "module" {
                docs_by_module.entry(m).or_default().push(f);
            }
            if *t == "class" {
                docs_by_class.entry(m).or_default().push(f);
            }
        }
    }

    for docstring_data in inventory::iter::<LuaDocstringData>() {
        populate_from(docstring_data.data, &mut docs_by_module, &mut docs_by_class);
    }

    let out_file = File::open(out_path)?;
    if !out_file.metadata()?.is_dir() {
        bail!("Output path '{out_path}' must be a directory");
    }

    let out_path = PathBuf::from(out_path);
    for (module, doc_fns) in docs_by_module.iter() {
        let file_path = out_path.join(format!("Mod_{module}.lua"));
        let mut w = std::io::BufWriter::new(File::create(file_path)?);
        writeln!(w, "--- Module {module}")?;
        writeln!(w, "--- @module {module}\n")?;

        for doc_fn in doc_fns {
            writeln!(w, "{doc_fn}")?;
        }
    }

    for (class, doc_fns) in docs_by_class.iter() {
        let file_path = out_path.join(format!("Class_{class}.lua"));
        let mut w = std::io::BufWriter::new(File::create(file_path)?);
        writeln!(w, "--- Class {class}")?;
        writeln!(w, "--- @classmod {class}\n")?;

        for doc_fn in doc_fns {
            writeln!(w, "{doc_fn}")?;
        }
    }

    Ok(())
}
