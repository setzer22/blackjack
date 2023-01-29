// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{graph::graph_interop, prelude::graph::*, prelude::*};
use std::path::{Path, PathBuf};

use blackjack_engine::graph::{
    serialization::{RuntimeData, SerializedBjkGraph, SerializedBjkSnippet, SerializedUiData},
    DependencyKind, NodeDefinitions,
};
use egui_node_graph::PanZoom;

use super::gizmo_ui::UiNodeGizmoStates;

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

    let locked_gizmo_nodes = custom_state
        .gizmo_states
        .get_all_locked_nodes()
        .iter_cpy()
        .map(node_id_to_idx)
        .collect();

    serialized.set_ui_data(SerializedUiData {
        node_positions,
        node_order,
        locked_gizmo_nodes,
        pan: Vec2::new(pan.x, pan.y),
        zoom: editor_state.pan_zoom.zoom,
    });

    serialized.write_to_file(path)?;

    Ok(())
}

pub fn load(
    path: PathBuf,
    node_definitions: &NodeDefinitions,
    gizmo_states: &UiNodeGizmoStates,
) -> Result<(GraphEditorState, CustomGraphState)> {
    let serialized = SerializedBjkGraph::load_from_file(&path)?;
    let (runtime, ui_data, id_idx_mappings) = serialized.into_runtime()?;

    if ui_data.is_none() {
        bail!(
            "The file at {} doesn't have UI information. Cannot load.",
            path.to_string_lossy()
        )
    }
    let ui_data = ui_data.unwrap();

    let (graph, mapping) = graph_interop::blackjack_graph_to_ui_graph(
        &runtime.graph,
        &runtime.external_parameters,
        node_definitions,
    )?;
    let idx_to_node_id = |idx| mapping[id_idx_mappings.get_id(idx).expect("Should exist")];

    let node_order = ui_data.node_order.iter_cpy().map(idx_to_node_id).collect();

    let node_positions = ui_data
        .node_positions
        .iter()
        .enumerate()
        .map(|(idx, pos)| (idx_to_node_id(idx), egui::pos2(pos.x, pos.y)))
        .collect();

    let active_node = runtime.graph.default_node.map(|x| mapping[x]);

    // Restore locked gizmo state
    gizmo_states.restore_locked_nodes(ui_data.locked_gizmo_nodes.iter_cpy().map(idx_to_node_id));
    if let Some(n) = active_node {
        // Make sure to enable the gizmo for the current active node, in case it
        // wasn't locked.
        gizmo_states.node_is_active(n);
    }

    let mut promoted_params = HashMap::default();
    for (bjk_node_id, bjk_node) in &runtime.graph.nodes {
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
        selected_nodes: Default::default(),
        ongoing_box_selection: None,
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
        active_node,
        node_definitions: node_definitions.share(),
        gizmo_states: gizmo_states.share(),
        promoted_params,
    };

    Ok((editor_state, custom_state))
}

pub fn to_clipboard(
    editor_state: &GraphEditorState,
    custom_state: &CustomGraphState,
    nodes: &[NodeId],
) -> Result<String> {
    let (bjk_graph, mapping) =
        graph_interop::ui_graph_to_blackjack_graph(&editor_state.graph, custom_state)?;
    let external_param_values =
        graph_interop::extract_graph_params(&editor_state.graph, &bjk_graph, &mapping)?;

    let (mut snippet, id_map) = SerializedBjkSnippet::from_runtime(
        bjk_graph,
        external_param_values,
        &nodes.iter_cpy().map(|x| mapping[x]).collect_vec(),
    )?;

    let node_id_to_idx =
        |id: NodeId| -> usize { id_map.get_idx(mapping[id]).expect("Id should exist") };

    let mut positions = editor_state
        .node_positions
        .iter()
        .filter(|(n, _)| nodes.contains(n));
    let (_, first_n_pos) = positions
        .next()
        .ok_or_else(|| anyhow!("Cannot copy an empty selection"))?;
    let mut aabb = egui::Rect::from_min_max(*first_n_pos, *first_n_pos);
    for (_, pos) in positions {
        aabb.extend_with(*pos);
    }

    let origin = aabb.left_top().to_vec2();
    snippet.set_node_relative_positions(
        nodes
            .iter_cpy()
            .map(|n| {
                let pos = editor_state
                    .node_positions
                    .get(n)
                    .copied()
                    .unwrap_or(egui::Pos2::ZERO);
                (node_id_to_idx(n), pos - origin)
            })
            .sorted_by_key(|(idx, _)| *idx)
            .map(|(_, pos)| glam::Vec2::new(pos.x, pos.y))
            .collect_vec(),
    );
    snippet.into_string()
}

pub fn parse_clipboard_snippet(clipboard_contents: &str) -> Result<SerializedBjkSnippet> {
    SerializedBjkSnippet::load_from_string(clipboard_contents)
}

pub fn from_clipboard(
    editor_state: &mut GraphEditorState,
    custom_state: &mut CustomGraphState,
    snippet: SerializedBjkSnippet,
    cursor_pos: egui::Pos2,
) -> Result<()> {
    // NOTE: This destructuring is added for future compatibility. We don't want
    // to forget updating this function when new things are added to the custom
    // state. Any fields that require special handling will be annotated below
    let CustomGraphState {
        run_side_effect: _,
        active_node: _,
        node_definitions: _,
        promoted_params: _,
        gizmo_states: _,
    } = custom_state;
    let GraphEditorState {
        // This is updated by `append_snippet_to_existing_ui_graph`
        graph: _,
        // Need to update this below, otherwise it will panic because new nodes
        // have been added.
        node_order: _,
        connection_in_progress: _,
        // The newly created nodes will become selected after a paste
        selected_nodes: _,
        ongoing_box_selection: _,
        // Need to update this, since new nodes have been added
        node_positions: _,
        node_finder: _,
        pan_zoom: _,
        _user_state: _,
    } = editor_state;

    let (rt_data, relative_node_positions, id_map) = snippet.into_runtime()?;

    let node_mapping = graph_interop::append_snippet_to_existing_ui_graph(
        &mut editor_state.graph,
        &rt_data.snippet,
        &rt_data.external_parameters,
        &custom_state.node_definitions,
    );

    editor_state.selected_nodes.clear();
    if let Some(positions) = relative_node_positions {
        for (idx, position) in positions.iter().enumerate() {
            let node_id = node_mapping[id_map.get_id(idx)?];
            let node_pos =
                cursor_pos + egui::vec2(position.x, position.y) - editor_state.pan_zoom.pan;
            editor_state.node_positions.insert(node_id, node_pos);
            editor_state.node_order.push(node_id);
            editor_state.selected_nodes.push(node_id);
        }
    } else {
        bail!("No node positions in snippet. Cannot paste")
    }

    Ok(())
}
