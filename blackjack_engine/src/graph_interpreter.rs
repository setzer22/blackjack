use mlua::ToLua;

use crate::graph::{BjkGraph, BjkNodeId, BlackjackValue};
use crate::lua_engine::{ProgramResult, RenderableThing};
use crate::prelude::*;

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ExternalParameter {
    pub node_id: BjkNodeId,
    pub param_name: String,
}

impl ExternalParameter {
    pub fn new(node_id: BjkNodeId, param_name: String) -> Self {
        Self {
            node_id,
            param_name,
        }
    }
}

#[derive(Debug, Default)]
pub struct ExternalParameterValues(pub HashMap<ExternalParameter, BlackjackValue>);

pub struct InterpreterContext<'a, 'lua> {
    outputs_cache: HashMap<BjkNodeId, mlua::Table<'lua>>,
    external_param_values: &'a ExternalParameterValues,
}

pub fn run_graph<'lua>(
    lua: &'lua mlua::Lua,
    graph: &BjkGraph,
    final_node: BjkNodeId,
    external_param_values: &ExternalParameterValues,
) -> Result<ProgramResult> {
    let mut context = InterpreterContext {
        outputs_cache: Default::default(),
        external_param_values,
    };

    // Ensure the outputs cache is populated.
    run_node(lua, graph, &mut context, final_node)?;

    let renderable = if let Some(return_value) = &graph.nodes[final_node].return_value {
        let output = context
            .outputs_cache
            .get(&final_node)
            .expect("Final node should be in the outputs cache");
        Some(RenderableThing::from_lua_value(
            output.get(return_value.as_str())?,
        )?)
    } else {
        None
    };

    Ok(ProgramResult { renderable })
}

pub fn run_node<'lua>(
    lua: &'lua mlua::Lua,
    graph: &BjkGraph,
    ctx: &mut InterpreterContext<'_, 'lua>,
    node_id: BjkNodeId,
) -> Result<()> {
    let node = &graph.nodes[node_id];

    // Stores the arguments that will be sent to this node's `op` fn
    let input_map = lua.create_table()?;

    // Compute the values for dependent nodes and populate the output cache.
    for input in &node.inputs {
        match &input.kind {
            crate::graph::DependencyKind::Connection { node, param_name } => {
                // Make sure the value is there by running the node.
                let cached_output_map = if let Some(cached) = ctx.outputs_cache.get(node) {
                    cached
                } else {
                    run_node(lua, graph, ctx, *node)?;
                    ctx.outputs_cache
                        .get(node)
                        .expect("Cache should be populated after calling run_node.")
                };

                input_map.set(
                    input.name.as_str(),
                    cached_output_map.get::<_, mlua::Value>(param_name.as_str())?,
                )?;
            }
            crate::graph::DependencyKind::Computed(_) => todo!(),
            crate::graph::DependencyKind::External => {
                let ext = ExternalParameter::new(node_id, input.name.clone());
                let val = ctx.external_param_values.0.get(&ext).ok_or_else(|| {
                    anyhow!(
                        "Could not retrieve external parameter named '{}' from node {}",
                        &input.name,
                        node_id.display_id(),
                    )
                })?;
                input_map.set(input.name.as_str(), val.clone().to_lua(lua)?)?;
            }
        }
    }

    let op_name = &node.op_name;
    let fn_code = format!(
        "return function(args) return require('node_library'):callNode('{op_name}', args) end"
    );
    let lua_fn: mlua::Function = lua.load(&fn_code).eval()?;
    let outputs = match lua_fn.call(input_map)? {
        mlua::Value::Table(t) => t,
        other => {
            bail!("A node's `op` function should always return a table, got {other:?}");
        }
    };

    ctx.outputs_cache.insert(node_id, outputs);

    Ok(())
}
