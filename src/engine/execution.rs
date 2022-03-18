use crate::prelude::*;
use halfedge::selection::SelectionExpression;
use mlua::{Lua, FromLua, Function, Table};

use super::lua_stdlib::{self, EngineValue};
use crate::{graph::graph_compiler2::CompiledProgram, prelude::graph::Graph};

pub fn extract_params<'lua>(
    lua: &'lua Lua,
    graph: &Graph,
    compiled: &CompiledProgram,
) -> Result<mlua::Table<'lua>> {
    let table = lua.create_table()?;
    for const_param in &compiled.const_parameters {
        let id = const_param.id;
        let input = graph.get_input(id);
        let ident = const_param.ident_str(graph)?;
        let value = match input.value() {
            crate::prelude::graph::ValueType::None => {
                Err(anyhow!("Cannot use constant value for non-existing type"))
            }
            crate::prelude::graph::ValueType::Vector(v) => {
                Ok(EngineValue::Vec3(lua_stdlib::Vec3(*v)))
            }
            crate::prelude::graph::ValueType::Scalar { value, .. } => {
                Ok(EngineValue::Scalar(*value))
            }
            crate::prelude::graph::ValueType::Selection { selection, .. } => Ok(
                EngineValue::Selection(selection.clone().unwrap_or(SelectionExpression::None)),
            ),
            crate::prelude::graph::ValueType::Enum { values, selection } => Ok(
                EngineValue::String(values[selection.unwrap_or(0) as usize].clone()),
            ),
            crate::prelude::graph::ValueType::NewFile { path } => {
                Ok(EngineValue::Path(lua_stdlib::Path(
                    path.as_ref()
                        .ok_or_else(|| anyhow!("Path not set"))?
                        .clone(),
                )))
            }
        }?;
        table.set(ident, value)?;
    }
    Ok(table)
}

pub fn run_program<'lua>(
    lua: &'lua Lua,
    compiled: &CompiledProgram,
    input: Table<'lua>,
) -> Result<HalfEdgeMesh> {
    lua.load(&compiled.lua_program).exec()?;
    let entry_point : Function = lua.globals().get("main")?;
    let mesh = entry_point.call::<_, HalfEdgeMesh>(input).map_err(|err| anyhow!("{}", err))?;
    Ok(mesh)
}

