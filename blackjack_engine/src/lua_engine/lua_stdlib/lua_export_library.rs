// Copyright (C) 2022 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

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
