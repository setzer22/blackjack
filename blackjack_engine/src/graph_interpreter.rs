// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use mlua::{Table, ToLua};
use slotmap::SecondaryMap;

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
    /// If not present, means all gizmo computations are skipped
    gizmo_state: Option<SecondaryMap<BjkNodeId, GizmoState>>,
    /// Stores the gizmo outputs for each node. This is not filled if
    /// gizmo_state is None.
    gizmo_outputs: &'a mut SecondaryMap<BjkNodeId, Vec<BlackjackGizmo>>,
}

#[derive(Clone, Debug, Default)]
pub struct GizmoState {
    pub active_gizmos: Option<Vec<BlackjackGizmo>>,
    pub gizmos_changed: bool,
}

pub fn run_graph(
    lua: &mlua::Lua,
    graph: &BjkGraph,
    target_node: BjkNodeId,
    mut external_param_values: ExternalParameterValues,
    node_definitions: &NodeDefinitions,
    gizmos_state: Option<SecondaryMap<BjkNodeId, GizmoState>>,
) -> Result<ProgramResult> {
    let gizmos_enabled = gizmos_state.is_some();

    let mut gizmo_outputs = Default::default();
    let mut context = InterpreterContext {
        outputs_cache: Default::default(),
        external_param_values: &mut external_param_values,
        node_definitions,
        gizmo_state: gizmos_state,
        gizmo_outputs: &mut gizmo_outputs,
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

    // Used to allow the gizmo input function to update a node's parameters.
    // This is None when gizmos don't run to optimize performance
    let mut referenced_external_params = if ctx.gizmo_state.is_some() {
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

    // This special value is injected into the inputs to signal nodes that the
    // gizmos are being processed. This is useful to let nodes optimize out
    // parts of the computation when they're running on a game engine.
    if ctx.gizmo_state.is_some() {
        input_map.set("__gizmos_enabled", true)?;
    }

    let node_table = lua
        .load(&(format!("require('node_library'):getNode('{op_name}')")))
        .eval::<mlua::Table>()?;

    struct GizmoFns<'lua> {
        update_params_fn: mlua::Function<'lua>,
        update_gizmos_fn: mlua::Function<'lua>,
        affected_params_fn: mlua::Function<'lua>,
    }

    struct GizmoDescriptor<'lua> {
        gizmos_changed: bool,
        data: Option<BlackjackGizmo>,
        fns: GizmoFns<'lua>,
    }

    // The data for each of the input gizmos. If this is the empty vec, then gizmos are disabled.
    let mut gizmo_descriptors: Vec<GizmoDescriptor> = (|| -> Result<_> {
        if node_def.has_gizmo {
            if let Some(gizmos_state) = &mut ctx.gizmo_state {
                // NOTE: We remove the input slotmap because each node only
                // needs the data from its own gizmos and all nodes are run
                // exactly once.
                if let Some(gizmo_data) = gizmos_state.remove(node_id) {
                    let gizmos_table: mlua::Table = node_table
                        .get("gizmos")
                        .map_err(|err| anyhow!("Expected node to have gizmos table. {err}"))?;

                    let mut gizmo_descriptors = Vec::<GizmoDescriptor>::new();
                    for (i, gizmo_descr) in
                        gizmos_table.sequence_values::<mlua::Table>().enumerate()
                    {
                        let gizmo_descr = gizmo_descr?;
                        macro_rules! get_fn {
                            ($name:expr) => {
                                gizmo_descr.get($name).map_err(|err| {
                                    anyhow!("Missing '{}' in gizmos table. {err}", $name)
                                })?
                            };
                        }

                        gizmo_descriptors.push(GizmoDescriptor {
                            data: gizmo_data.active_gizmos.as_ref().map(|v| v[i].clone()),
                            gizmos_changed: gizmo_data.gizmos_changed,
                            fns: GizmoFns {
                                update_params_fn: get_fn!("update_params"),
                                update_gizmos_fn: get_fn!("update_gizmos"),
                                affected_params_fn: get_fn!("affected_params"),
                            },
                        });
                    }
                    return Ok(gizmo_descriptors);
                }
            }
        }
        Ok(Vec::new())
    })()?;

    // When a gizmo affects parameters, the gizmo will be disabled if all the
    // parameters are connected. Nodes can return nil from the `affected_params`
    // function to disable this behavior.
    let enabled_gizmos = gizmo_descriptors
        .iter()
        .map(|descr| {
            if let Some(affected_params) = descr
                .fns
                .affected_params_fn
                .call::<_, Option<Vec<String>>>(())?
            {
                for input in node.inputs.iter() {
                    if affected_params.contains(&input.name) {
                        match &input.kind {
                            crate::graph::DependencyKind::External { .. } => return Ok(true),
                            crate::graph::DependencyKind::Connection { .. } => {}
                        }
                    }
                }
                Ok(false)
            } else {
                Ok(true)
            }
        })
        .collect::<Result<Vec<bool>>>()?;

    // Run pre-gizmo
    for it in gizmo_descriptors.iter().zip(&enabled_gizmos) {
        if let (
            GizmoDescriptor {
                gizmos_changed: true,
                data: Some(gizmo_in),
                fns: GizmoFns {
                    update_params_fn, ..
                },
            },
            true,
        ) = it
        {
            // Update params
            // Patch the input map, running the gizmo function
            let input_gizmo = gizmo_in.clone().to_lua(lua)?;
            let new_input_map = update_params_fn
                .call::<_, Table>((input_map, input_gizmo))
                .map_err(|err| {
                    anyhow!(
                        "A node's update_params gizmo callback should return an updated parameter list as a table. {err}"
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
                        "The gizmos input function modified a parameter in an illegal way: {err}"
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
    for (gz_descr, enabled) in gizmo_descriptors.iter_mut().zip(&enabled_gizmos) {
        let updated_gizmo = enabled
            .then(|| -> anyhow::Result<_> {
                let gizmo = gz_descr
                    .data
                    .as_mut()
                    .map(|gz| gz.clone().to_lua(lua))
                    .transpose()?
                    .unwrap_or(mlua::Value::Nil);

                gz_descr
                    .fns
                    .update_gizmos_fn
                    .call::<_, BlackjackGizmo>((input_map.clone(), gizmo, outputs.clone()))
                    .map_err(|err| {
                        anyhow!("A node's gizmo outputs function should return a new gizmo. {err}")
                    })
            })
            .transpose()?;
        ctx.gizmo_outputs
            .entry(node_id)
            .unwrap()
            .or_default()
            // When gizmos are disabled, we push a `None` value. This will be
            // converted into `nil` when that gizmo is enabled again and its
            // value is pushed to Lua.
            .push(updated_gizmo.unwrap_or(BlackjackGizmo::None));
    }

    Ok(())
}
