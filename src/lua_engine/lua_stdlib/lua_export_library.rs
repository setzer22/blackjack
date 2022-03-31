use super::*;

pub fn load(lua: &Lua) -> anyhow::Result<()> {
    let globals = lua.globals();
    let export = lua.create_table()?;
    globals.set("Export", export.clone())?;

    lua_fn!(lua, export, "wavefront_obj", |mesh: AnyUserData,
                                           path: Path|
     -> () {
        let mesh = mesh.borrow::<HalfEdgeMesh>()?;
        mesh.to_wavefront_obj(path.0).map_lua_err()?;
        Ok(())
    });

    Ok(())
}