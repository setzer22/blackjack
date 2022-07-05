// Copyright (C) 2022 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{path::PathBuf, str::FromStr};

use super::*;

pub fn load(lua: &Lua) -> anyhow::Result<()> {
    let globals = lua.globals();
    let blackjack = lua.create_table()?;
    globals.set("Blackjack", blackjack.clone())?;

    globals.set(
        "Vec3",
        lua.create_function(|_, (x, y, z)| Ok(mlua::Value::Vector(x, y, z)))?,
    )?;

    lua_fn!(
        lua,
        blackjack,
        "selection",
        |expr: mlua::String| -> SelectionExpression {
            SelectionExpression::parse(expr.to_str()?).map_lua_err()
        }
    );

    lua_fn!(lua, blackjack, "path", |path: mlua::String| -> Path {
        Ok(Path(PathBuf::from_str(path.to_str()?).map_err(|err| {
            mlua::Error::RuntimeError(format!("Invalid path: {:?}", err))
        })?))
    });

    lua_fn!(lua, blackjack, "perlin", || -> PerlinNoise {
        Ok(PerlinNoise(noise::Perlin::new()))
    });
    lua_fn!(lua, blackjack, "print_vertex", |id: VertexId| -> () {
        println!("{:?}", id);
        Ok(())
    });

    lua_fn!(lua, blackjack, "mesh", || -> HalfEdgeMesh {
        Ok(HalfEdgeMesh::new())
    });

    lua.globals().set(
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
