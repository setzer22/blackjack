// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::Index;

use super::node_graph::{
    data_type_to_input_param_kind, default_shown_inline, CustomGraphState, DataTypeUi, Graph,
    NodeData, ValueTypeUi,
};

use crate::prelude::*;
use blackjack_engine::{
    graph::{
        BjkGraph, BjkNode, BjkNodeId, BjkSnippet, BlackjackValue, DependencyKind, NodeDefinitions,
    },
    graph_interpreter::{ExternalParameter, ExternalParameterValues},
};
use egui_node_graph::{InputId, NodeId, OutputId};
use slotmap::SecondaryMap;

#[derive(Clone, Debug, Default)]
pub struct NodeMapping(
    SecondaryMap<NodeId, BjkNodeId>,
    SecondaryMap<BjkNodeId, NodeId>,
);
impl NodeMapping {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn insert(&mut self, node_id: NodeId, bjk_node_id: BjkNodeId) {
        self.0.insert(node_id, bjk_node_id);
        self.1.insert(bjk_node_id, node_id);
    }
}
impl Index<NodeId> for NodeMapping {
    type Output = BjkNodeId;
    fn index(&self, index: NodeId) -> &Self::Output {
        &self.0[index]
    }
}
impl Index<BjkNodeId> for NodeMapping {
    type Output = NodeId;
    fn index(&self, index: BjkNodeId) -> &Self::Output {
        &self.1[index]
    }
}

pub fn ui_graph_to_blackjack_graph(
    graph: &Graph,
    custom_state: &CustomGraphState,
) -> Result<(BjkGraph, NodeMapping)> {
    let mut bjk_graph = BjkGraph::new();
    let mut mapping = NodeMapping::new();
    let mut input_names = SecondaryMap::<InputId, &str>::new();
    let mut output_names = SecondaryMap::<OutputId, &str>::new();

    for (node_id, node) in &graph.nodes {
        let node_def = custom_state
            .node_definitions
            .node_def(&node.user_data.op_name)
            .ok_or_else(|| anyhow!("Node definition not found for {}", &node.user_data.op_name))?;

        let bjk_id = bjk_graph.add_node(node.user_data.op_name.clone(), node_def.returns.clone());
        mapping.insert(node_id, bjk_id);

        for (input_name, input_id) in &node.inputs {
            bjk_graph.add_input(
                bjk_id,
                input_name,
                graph.inputs[*input_id].typ.0,
                custom_state.promoted_params.get(input_id).cloned(),
            )?;
            input_names.insert(*input_id, input_name);
        }
        for (output_name, output_id) in &node.outputs {
            bjk_graph.add_output(bjk_id, output_name, graph.outputs[*output_id].typ.0)?;
            output_names.insert(*output_id, output_name);
        }
    }

    for (input, output) in &graph.connections {
        let input_name = input_names[input];
        let output_name = output_names[*output];

        let input_node_id = mapping[graph[input].node];
        let output_node_id = mapping[graph[*output].node];

        bjk_graph.add_connection(output_node_id, output_name, input_node_id, input_name)?;
    }

    bjk_graph.default_node = custom_state.active_node.map(|x| mapping[x]);

    Ok((bjk_graph, mapping))
}

pub fn add_ui_node_from_bjk_node(
    graph: &mut Graph,
    bjk_node_id: BjkNodeId,
    bjk_node: &BjkNode,
    mapping: &mut NodeMapping,
    node_definitions: &NodeDefinitions,
) {
    let new_id = graph.add_node(
        if let Some(node_def) = node_definitions.node_def(&bjk_node.op_name) {
            node_def.label.clone()
        } else {
            "⚠ Unknown".into()
        },
        NodeData {
            op_name: bjk_node.op_name.clone(),
        },
        |_, _| { /* Params added later */ },
    );
    mapping.insert(new_id, bjk_node_id);
}

pub fn set_inputs_outputs_from_bjk_node(
    graph: &mut Graph,
    bjk_node_id: BjkNodeId,
    bjk_node: &BjkNode,
    mapping: &mut NodeMapping,
    node_definitions: &NodeDefinitions,
    external_parameters: &Option<ExternalParameterValues>,
) {
    for bjk_input in &bjk_node.inputs {
        graph.add_input_param(
            mapping[bjk_node_id],
            bjk_input.name.clone(),
            DataTypeUi(bjk_input.data_type),
            {
                let get_runtime_val = || -> BlackjackValue {
                    // Try to get the value from the external parameters
                    if let Some(ext) = external_parameters {
                        let param = &ExternalParameter {
                            node_id: bjk_node_id,
                            param_name: bjk_input.name.clone(),
                        };
                        if let Some(val) = ext.0.get(param) {
                            return val.clone();
                        }
                    }
                    // Otherwise, try to get it from the node definition's default value
                    if let Some(node_def) = node_definitions.node_def(&bjk_node.op_name) {
                        if let Some(def) = node_def
                            .inputs
                            .iter()
                            .find(|input| input.name == bjk_input.name)
                        {
                            return def.default_value();
                        }
                    }
                    // If all else fails, return the default for the datatype.
                    bjk_input.data_type.default_value()
                };
                ValueTypeUi(get_runtime_val())
            },
            data_type_to_input_param_kind(bjk_input.data_type),
            default_shown_inline(),
        );
    }

    for bjk_output in &bjk_node.outputs {
        graph.add_output_param(
            mapping[bjk_node_id],
            bjk_output.name.clone(),
            DataTypeUi(bjk_output.data_type),
        );
    }
}

pub fn set_ui_connections_from_bjk_node(
    graph: &mut Graph,
    bjk_node_id: BjkNodeId,
    bjk_node: &BjkNode,
    mapping: &mut NodeMapping,
) {
    for bjk_input in &bjk_node.inputs {
        match &bjk_input.kind {
            DependencyKind::Connection { node, param_name } => {
                let out_node_id = mapping[*node];
                let out_id = graph[out_node_id]
                    .get_output(param_name)
                    .expect("Param should exist, we just added it.");

                let in_node_id = mapping[bjk_node_id];
                let in_id = graph[in_node_id]
                    .get_input(&bjk_input.name)
                    .expect("Param should exist, we just added it.");

                graph.add_connection(out_id, in_id);
            }
            DependencyKind::External { .. } => {}
        }
    }
}

pub fn blackjack_graph_to_ui_graph(
    bjk_graph: &BjkGraph,
    external_parameters: &Option<ExternalParameterValues>,
    node_definitions: &NodeDefinitions,
) -> Result<(Graph, NodeMapping)> {
    // Create the graph and the id mappings
    let mut graph = Graph::new();
    let mut mapping = NodeMapping::new();
    let BjkGraph {
        nodes: bjk_nodes,
        default_node: _,
    } = bjk_graph;

    // Fill in the nodes in a first pass
    for (bjk_node_id, bjk_node) in bjk_nodes {
        add_ui_node_from_bjk_node(
            &mut graph,
            bjk_node_id,
            bjk_node,
            &mut mapping,
            node_definitions,
        );
    }

    // Then, define inputs / outputs in a second pass.
    for (bjk_node_id, bjk_node) in bjk_nodes {
        set_inputs_outputs_from_bjk_node(
            &mut graph,
            bjk_node_id,
            bjk_node,
            &mut mapping,
            node_definitions,
            external_parameters,
        );
    }

    // Finally, define connections in a third pass.
    for (bjk_node_id, bjk_node) in bjk_nodes {
        set_ui_connections_from_bjk_node(&mut graph, bjk_node_id, bjk_node, &mut mapping);
    }

    Ok((graph, mapping))
}

/// Adds the provided `snippet` into the `graph`. The returned `NodeMaping`
/// contains *only* the ids for newly created nodes.
pub fn append_snippet_to_existing_ui_graph(
    graph: &mut Graph,
    snippet: &BjkSnippet,
    external_parameters: &Option<ExternalParameterValues>,
    node_definitions: &NodeDefinitions,
) -> NodeMapping {
    let mut mapping = NodeMapping::new();

    // NOTE: There's a bit of @CopyPaste'd code here from
    // `blackjack_graph_to_ui_graph` above, but I'd rather have the two
    // functions separate to accomodate for potential future differences. All
    // the common bits have already been refactored into functions.

    // Add new nodes in a first pass
    for (bjk_node_id, bjk_node) in &snippet.nodes {
        add_ui_node_from_bjk_node(graph, bjk_node_id, bjk_node, &mut mapping, node_definitions);
    }

    // Then, define inputs / outputs in a second pass.
    for (bjk_node_id, bjk_node) in &snippet.nodes {
        set_inputs_outputs_from_bjk_node(
            graph,
            bjk_node_id,
            bjk_node,
            &mut mapping,
            node_definitions,
            external_parameters,
        );
    }

    // Finally, define connections in a third pass.
    for (bjk_node_id, bjk_node) in &snippet.nodes {
        set_ui_connections_from_bjk_node(graph, bjk_node_id, bjk_node, &mut mapping);
    }

    mapping
}

pub fn extract_graph_params(
    graph: &Graph,
    bjk_graph: &BjkGraph,
    mapping: &NodeMapping,
) -> Result<ExternalParameterValues> {
    let mut params = ExternalParameterValues::default();

    for (node_id, node) in &bjk_graph.nodes {
        for input in &node.inputs {
            if let DependencyKind::External { .. } = input.kind {
                let external_param = ExternalParameter::new(node_id, input.name.clone());
                let ui_node_id = mapping[node_id];
                let ui_input = graph[ui_node_id].get_input(&input.name)?;
                params
                    .0
                    .insert(external_param, graph[ui_input].value.0.clone());
            }
        }
    }

    Ok(params)
}

pub fn set_parameters_from_external_values(
    graph: &mut Graph,
    updated_values: ExternalParameterValues,
    mapping: NodeMapping,
) -> Result<()> {
    for (param, value) in updated_values.0 {
        let node_id = mapping[param.node_id];
        let input_id = graph[node_id].get_input(&param.param_name)?;

        let input = &mut graph[input_id];

        if input.typ.0.is_valid_value(&value) {
            graph[input_id].value = ValueTypeUi(value);
        }
    }
    Ok(())
}
