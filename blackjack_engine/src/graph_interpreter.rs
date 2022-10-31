use mlua::ToLua;

use crate::gizmos::BlackjackGizmo;
use crate::graph::{BjkGraph, BjkNodeId, BlackjackValue};
use crate::lua_engine::{ProgramResult, RenderableThing};
use crate::prelude::*;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
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

#[derive(Debug, Default, Clone)]
pub struct ExternalParameterValues(pub HashMap<ExternalParameter, BlackjackValue>);

pub struct InterpreterContext<'a, 'lua> {
    outputs_cache: HashMap<BjkNodeId, mlua::Table<'lua>>,
    /// The values for all the external parameters. Mutable reference because
    /// node gizmos may modify these values.
    external_param_values: &'a mut ExternalParameterValues,
    /// See `active_gizmos` on [`ProgramResult`]
    active_gizmos: &'a mut Vec<BlackjackGizmo>,
}

pub fn run_graph<'lua>(
    lua: &'lua mlua::Lua,
    graph: &BjkGraph,
    final_node: BjkNodeId,
    mut external_param_values: ExternalParameterValues,
) -> Result<ProgramResult> {
    let mut gizmos = Vec::new();
    let mut context = InterpreterContext {
        outputs_cache: Default::default(),
        external_param_values: &mut external_param_values,
        active_gizmos: &mut gizmos,
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

    Ok(ProgramResult {
        renderable,
        updated_values: external_param_values,
        active_gizmos: gizmos,
    })
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
            crate::graph::DependencyKind::External { promoted: _ } => {
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
