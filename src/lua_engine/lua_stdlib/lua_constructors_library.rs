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

    Ok(())
}