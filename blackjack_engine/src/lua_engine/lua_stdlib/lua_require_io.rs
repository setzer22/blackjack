use std::path::PathBuf;

use super::lua_node_libraries::LuaSourceFile;

/// This trait is used to abstract Lua file IO to allow different
/// implementations for different platforms. Some game engines, like Godot, use
/// packed binary data and offer their own `File` APIs to handle that
/// abstraction, using regular std::io in those cases would not work.
///
/// If you are writing an integration and your engine can load assets using
/// Rust's standard io functions, you can use the `StdLuaFileIo` default
/// implementation which uses the Rust standard library.
pub trait LuaFileIo {
    /// Returns an iterator over the paths of all the blackjack initialization
    /// scripts on the lua folder.
    ///
    /// The calling code does not care about the format of the returned paths.
    /// The values should be valid to call the `load_file_absolute` function,
    /// which in practice means they should be absolute paths.
    fn find_run_files(&self) -> Box<dyn Iterator<Item = String>>;

    /// Returns a [`LuaSourceFile`] with the contents of the file at a given
    /// `path`. The path will be treated as absolute.
    fn load_file_absolute(&self, path: &str) -> Result<LuaSourceFile, Box<dyn std::error::Error>>;

    /// Returns a [`LuaSourceFile`] with the contents of the file at a given
    /// `path`. The path is relative to $BLACKJACK_LUA/lib. This function will
    /// be used when Lua code calls the `require` function.
    fn load_file_require(&self, path: &str) -> Result<LuaSourceFile, Box<dyn std::error::Error>>;
}

pub struct StdLuaFileIo {
    pub base_folder: String,
}

impl LuaFileIo for StdLuaFileIo {
    fn find_run_files(&self) -> Box<dyn Iterator<Item = String>> {
        Box::new(
            walkdir::WalkDir::new(&self.base_folder)
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

    fn load_file_absolute(&self, path: &str) -> Result<LuaSourceFile, Box<dyn std::error::Error>> {
        Ok(LuaSourceFile {
            contents: std::fs::read_to_string(path)?,
            name: path.into(),
        })
    }

    fn load_file_require(&self, path: &str) -> Result<LuaSourceFile, Box<dyn std::error::Error>> {
        let mut path = PathBuf::from(&self.base_folder).join(path);
        if !path.ends_with(".lua") {
            path.push(".lua");
        }
        Ok(LuaSourceFile {
            contents: std::fs::read_to_string(&path)?,
            name: path.display().to_string(),
        })
    }
}
