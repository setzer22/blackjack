use mlua::{Table, ToLua};

use crate::gizmos::BlackjackGizmo;
use crate::graph::{BjkGraph, BjkNodeId, BlackjackValue, NodeDefinitions};
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
    node_definitions: &'a NodeDefinitions,
    target_node: BjkNodeId,
    gizmos_enabled: bool,
    gizmo_config: GizmoConfig,
    gizmo_outputs: &'a mut Vec<BlackjackGizmo>,
}

pub enum GizmoConfig {
    IgnoreGizmos,
    RunGizmosInOut(Vec<BlackjackGizmo>),
    RinGizmoOut,
}

pub fn run_graph<'lua>(
    lua: &'lua mlua::Lua,
    graph: &BjkGraph,
    target_node: BjkNodeId,
    mut external_param_values: ExternalParameterValues,
    node_definitions: &NodeDefinitions,
    gizmo_config: GizmoConfig,
) -> Result<ProgramResult> {
    let gizmos_enabled = matches!(
        &gizmo_config,
        GizmoConfig::RinGizmoOut | GizmoConfig::RunGizmosInOut(_)
    );

    let mut gizmo_outputs = Vec::new();
    let mut context = InterpreterContext {
        outputs_cache: Default::default(),
        external_param_values: &mut external_param_values,
        target_node,
        node_definitions,
        gizmo_config,
        gizmo_outputs: &mut gizmo_outputs,
        gizmos_enabled,
    };

    // Ensure the outputs cache is populated.
    run_node(lua, graph, &mut context, target_node)?;

    let renderable = if let Some(return_value) = &graph.nodes[target_node].return_value {
        let output = context
            .outputs_cache
            .get(&target_node)
            .expect("Final node should be in the outputs cache");
        Some(RenderableThing::from_lua_value(
            output.get(return_value.as_str())?,
        )?)
    } else {
        None
    };

    Ok(ProgramResult {
        renderable,
        updated_gizmos: if gizmos_enabled {
            Some(gizmo_outputs)
        } else {
            None
        },
        updated_values: external_param_values,
    })
}

pub fn run_node<'lua>(
    lua: &'lua mlua::Lua,
    graph: &BjkGraph,
    ctx: &mut InterpreterContext<'_, 'lua>,
    node_id: BjkNodeId,
) -> Result<()> {
    let node = &graph.nodes[node_id];
    let op_name = &node.op_name;
    let node_def = ctx
        .node_definitions
        .node_def(op_name)
        .ok_or_else(|| anyhow!("Node definition not found for {op_name}"))?;

    // Stores the arguments that will be sent to this node's `op` fn
    let mut input_map = lua.create_table()?;

    // Used to allow the gizmo_in function to update a node's parameters. This
    // is None when gizmos don't run to optimize performance
    let mut referenced_external_params = if ctx.gizmos_enabled {
        Some(Vec::<ExternalParameter>::new())
    } else {
        None
    };

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
                if let Some(m) = &mut referenced_external_params {
                    m.push(ext);
                }
            }
        }
    }

    let node_table = lua
        .load(&(format!("require('node_library'):getNode('{op_name}')")))
        .eval::<mlua::Table>()?;

    // Run pre-gizmo
    if ctx.gizmos_enabled && node_id == ctx.target_node && node_def.has_gizmo {
        if let GizmoConfig::RunGizmosInOut(gizmos_in) = &ctx.gizmo_config {
            let gizmo_in_fn: mlua::Function = node_table
                .get("gizmo_in")
                .map_err(|err| anyhow!("Node with gizmo should have 'gizmo_in'. {err}"))?;

            // Patch the input map, running the gizmo function
            let input_gizmos = gizmos_in.clone().to_lua(lua)?;
            let new_input_map = gizmo_in_fn
                .call::<_, Table>((input_map, input_gizmos))
                .map_err(|err| {
                    anyhow!(
                        "A node's gizmo_in callback should return an
                    updated parameter list as a table. {err}"
                    )
                })?;
            input_map = new_input_map;

            // Write the inputs that were returned to lua back to the
            // external_parameter_values in the context. This will then be sent
            // as part of the program output, to communicate to the integration
            // that parameters for a node have changed.
            let referenced_external_params = referenced_external_params
                .as_ref()
                .expect("When gizmos run, this should be defined");
            for param in referenced_external_params.iter() {
                let new_val = input_map
                    .get::<_, BlackjackValue>(param.param_name.clone())
                    .map_err(|err| {
                        anyhow!(
                            "The gizmos_in function modified a parameter in an illegal way: {err}"
                        )
                    })?;
                *ctx.external_param_values
                    .0
                    .get_mut(param)
                    .expect("Should be there") = new_val;
            }
        }
    }

    // Run node 'op'
    let op_fn: mlua::Function = node_table
        .get("op")
        .map_err(|err| anyhow!("Node should always have an 'op'. {err}"))?;
    let outputs = match op_fn.call(input_map.clone())? {
        mlua::Value::Table(t) => t,
        other => {
            bail!("A node's `op` function should always return a table, got {other:?}");
        }
    };

    ctx.outputs_cache.insert(node_id, outputs.clone());

    // Run post-gizmo
    if ctx.gizmos_enabled && node_id == ctx.target_node && node_def.has_gizmo {
        let gizmo_out_fn: mlua::Function = node_table
            .get("gizmo_out")
            .map_err(|err| anyhow!("Node with gizmo should have 'gizmo_out'. {err}"))?;

        let input_gizmos = match &ctx.gizmo_config {
            GizmoConfig::RunGizmosInOut(gizmos_in) => gizmos_in.clone().to_lua(lua)?,
            _ => mlua::Value::Nil,
        };

        let gizmos: Vec<BlackjackGizmo> = gizmo_out_fn
            .call((input_map, input_gizmos, outputs))
            .map_err(|err| {
                anyhow!("A node's gizmo_out function should return a sequence of gizmos. {err}")
            })?;

        *ctx.gizmo_outputs = gizmos;
    }

    Ok(())
}
