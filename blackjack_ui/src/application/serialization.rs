// Copyright (C) 2022 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    graph::graph_interop::{self, NodeMapping},
    prelude::graph::*,
    prelude::*,
};
use std::path::{Path, PathBuf};

use blackjack_engine::{
    graph::{
        serialization::{RuntimeData, SerializedBjkGraph, SerializedUiData},
        BjkGraph, BjkNodeId, BlackjackValue, DependencyKind, InputParameter, NodeDefinitions,
    },
    graph_interpreter::{ExternalParameter, ExternalParameterValues},
};
use egui_node_graph::PanZoom;

pub fn save(
    editor_state: &GraphEditorState,
    custom_state: &CustomGraphState,
    path: impl AsRef<Path>,
) -> Result<()> {
    let (bjk_graph, mapping) =
        graph_interop::ui_graph_to_blackjack_graph(&editor_state.graph, custom_state)?;
    let external_param_values =
        graph_interop::extract_graph_params(&editor_state.graph, &bjk_graph, &mapping)?;
    let (mut serialized, id_map) =
        blackjack_engine::graph::serialization::SerializedBjkGraph::from_runtime(RuntimeData {
            graph: bjk_graph,
            external_parameters: Some(external_param_values),
        })?;

    let node_id_to_idx =
        |id: NodeId| -> usize { id_map.get_idx(mapping[id]).expect("Id should exist") };

    let node_positions = editor_state
        .node_positions
        .iter()
        .map(|(node_id, pos2)| (node_id_to_idx(node_id), glam::Vec2::new(pos2.x, pos2.y)))
        .sorted_by_key(|(idx, _pos)| *idx)
        .map(|(_idx, pos)| pos)
        .collect();

    let node_order = editor_state
        .node_order
        .iter_cpy()
        .map(node_id_to_idx)
        .collect();
    let pan = editor_state.pan_zoom.pan;

    serialized.set_ui_data(SerializedUiData {
        node_positions,
        node_order,
        active_node: custom_state.active_node.map(node_id_to_idx),
        pan: Vec2::new(pan.x, pan.y),
        zoom: editor_state.pan_zoom.zoom,
    });

    serialized.write_to_file(path)?;

    Ok(())
}

pub fn load(
    path: PathBuf,
    node_definitions: &NodeDefinitions,
) -> Result<(GraphEditorState, CustomGraphState)> {
    // TODO: REVIEW: Should at least move some of this code to graph_interop.rs,
    // where the other function is.

    let serialized = SerializedBjkGraph::load_from_file(&path)?;
    let (runtime, ui_data, id_idx_mappings) = serialized.to_runtime()?;
    if let Some(ui_data) = ui_data {
        // Create the graph and the id mappings
        let mut graph = Graph::new();
        let mut mapping = NodeMapping::new();
        let BjkGraph { nodes: bjk_nodes } = runtime.graph;

        // Fill in the nodes in a first pass
        for (bjk_node_id, bjk_node) in &bjk_nodes {
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

        // Then, define inputs / outputs in a second pass.
        for (bjk_node_id, bjk_node) in &bjk_nodes {
            for bjk_input in &bjk_node.inputs {
                graph.add_input_param(
                    mapping[bjk_node_id],
                    bjk_input.name.clone(),
                    DataTypeUi(bjk_input.data_type),
                    {
                        let external_parameters = &runtime.external_parameters;
                        let get_runtime_val = || -> BlackjackValue {
                            // Try to get the value from the external parameters
                            if let Some(ext) = external_parameters {
                                let param = &ExternalParameter {
                                    node_id: bjk_node_id,
                                    param_name: bjk_input.name.clone(),
                                };
                                if let Some(val) = ext.0.get(&param) {
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

        // Finally, define connections in a third pass.
        for (bjk_node_id, bjk_node) in &bjk_nodes {
            for bjk_input in &bjk_node.inputs {
                match &bjk_input.kind {
                    DependencyKind::Connection { node, param_name } => {
                        let out_node_id = mapping[*node];
                        let out_id = graph[out_node_id]
                            .get_output(&param_name)
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

        let idx_to_node_id = |idx| mapping[id_idx_mappings.get_id(idx).expect("Should exist")];

        let node_order = ui_data
            .node_order
            .iter()
            .map(|idx| idx_to_node_id(*idx))
            .collect();

        let node_positions = ui_data
            .node_positions
            .iter()
            .enumerate()
            .map(|(idx, pos)| (idx_to_node_id(idx), egui::pos2(pos.x, pos.y)))
            .collect();

        let mut promoted_params = HashMap::default();
        for (bjk_node_id, bjk_node) in &bjk_nodes {
            for bjk_input in &bjk_node.inputs {
                if let DependencyKind::External {
                    promoted: Some(promoted),
                } = &bjk_input.kind
                {
                    let input_id = graph[mapping[bjk_node_id]]
                        .get_input(&bjk_input.name)
                        .expect("Should exist");
                    promoted_params.insert(input_id, promoted.clone());
                }
            }
        }

        let editor_state = GraphEditorState {
            graph,
            node_order,
            connection_in_progress: None,
            selected_node: None,
            node_positions,
            node_finder: None,
            pan_zoom: PanZoom {
                pan: egui::vec2(ui_data.pan.x, ui_data.pan.y),
                zoom: ui_data.zoom,
            },
            _user_state: std::marker::PhantomData,
        };
        let custom_state = CustomGraphState {
            run_side_effect: None,
            active_node: ui_data.active_node.map(idx_to_node_id),
            node_definitions: node_definitions.share(),
            promoted_params,
        };

        Ok((editor_state, custom_state))
    } else {
        bail!(
            "The file at {} doesn't have UI information. Cannot load.",
            path.to_string_lossy()
        )
    }
}
