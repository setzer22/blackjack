
pub mod asset_manager {
    use std::io::Read;
    use std::fs::File;

    #[cfg(target_arch = "wasm32")]
    use include_dir::include_dir;
    use include_dir::Dir;

    pub fn get(path : &str) -> Option<Box<dyn Read>> {
        #[cfg(target_arch = "wasm32")]
        static COMPILED_ASSETS: Dir = include_dir!("$CARGO_MANIFEST_DIR/assets");
        #[cfg(not(target_arch = "wasm32"))]
        static COMPILED_ASSETS: Dir = Dir::new("", &[]);

        let path = if path.starts_with("./") {
            path.strip_prefix("./").unwrap()
        } else {
            path
        };
        let compiled_path = if path.starts_with("assets/") {
            path.strip_prefix("assets/").unwrap()
        } else {
            path
        };
        if let Some(file) = COMPILED_ASSETS.get_file(compiled_path) {
            return Some( Box::new( file.contents()) );
        }

        if let Ok(file) = File::open(path) {
            Some( Box::new( file ))
        } else {
            None
        }
    }
}
