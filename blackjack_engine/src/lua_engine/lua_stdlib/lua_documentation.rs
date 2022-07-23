use anyhow::{bail, Result};
use std::io::Write;
use std::{collections::BTreeMap, fs::File, path::PathBuf};

pub fn generate_lua_documentation(out_path: &str) -> Result<()> {
    let mut docs_by_module = BTreeMap::<&str, Vec<&str>>::new();

    fn populate_from<'a>(
        it: &'a [(&'a str, &'a str)],
        docs_by_module: &mut BTreeMap<&'a str, Vec<&'a str>>,
    ) {
        for (m, f) in it {
            docs_by_module.entry(m).or_default().push(f);
        }
    }

    populate_from(
        crate::mesh::halfedge::edit_ops::lua_fns::__blackjack_lua_docstrings,
        &mut docs_by_module,
    );

    let out_file = File::open(&out_path)?;
    if !out_file.metadata()?.is_dir() {
        bail!("Output path '{out_path}' must be a directory");
    }

    let out_path = PathBuf::from(out_path);
    for (module, doc_fns) in docs_by_module.iter() {
        let file_path = out_path.join(format!("{module}.lua"));
        let mut w = std::io::BufWriter::new(File::create(file_path)?);
        writeln!(w, "--- Module {module}")?;
        writeln!(w, "--- @module {module}\n")?;

        for doc_fn in doc_fns {
            writeln!(w, "{doc_fn}")?;
        }
    }

    Ok(())
}

pub fn handle_ldoc_cmdline_arg() -> bool {
    let args: Vec<String> = std::env::args().collect();
    if let Some(arg) = args.get(1) {
        if arg == "--generate-ldoc" {
            if let Some(out_path) = args.get(2) {
                if let Err(err) = generate_lua_documentation(out_path) {
                    eprintln!("LDoc generation error: {err}");
                }
            } else {
                eprintln!(
                    "When --generate-ldoc is provided, second argument must be the output path"
                );
            }
            return true;
        }
    }
    false
}
