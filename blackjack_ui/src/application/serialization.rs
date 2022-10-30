// Copyright (C) 2022 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{graph::graph_interop, prelude::graph::*, prelude::*};
use std::path::PathBuf;

use blackjack_engine::graph::{serialization::RuntimeData, NodeDefinitions};
use egui_node_graph::PanZoom;
use serde::{Deserialize, Serialize};
use slotmap::SecondaryMap;

/// We don't serialize the whole editor state. Instead, we serialize just a few
/// select fields.
#[derive(Serialize, Deserialize)]
struct SerializedEditorState {
    pub graph: graph::Graph,
    pub node_order: Option<Vec<NodeId>>,
    pub active_node: Option<NodeId>,
    pub node_positions: SecondaryMap<NodeId, egui::Pos2>,
    pub pan_zoom: PanZoom,
}

impl SerializedEditorState {
    pub fn from_state(editor_state: &GraphEditorState, custom_state: &CustomGraphState) -> Self {
        SerializedEditorState {
            graph: editor_state.graph.clone(),
            node_order: Some(editor_state.node_order.clone()),
            active_node: custom_state.active_node,
            node_positions: editor_state.node_positions.clone(),
            pan_zoom: editor_state.pan_zoom,
        }
    }

    pub fn into_state(self) -> (GraphEditorState, CustomGraphState) {
        let custom_state = CustomGraphState {
            run_side_effect: None,
            active_node: self.active_node,
            node_definitions: NodeDefinitions::default(), // TODO: HACK: This won't work
            promoted_params: HashMap::default(),          // TODO: HACK: This won't work
        };

        let mut editor_state = GraphEditorState::new(1.0);
        editor_state.graph = self.graph;
        editor_state.node_order = self
            .node_order
            .unwrap_or_else(|| editor_state.graph.iter_nodes().collect());
        editor_state.node_positions = self.node_positions;
        editor_state.pan_zoom = self.pan_zoom;

        (editor_state, custom_state)
    }
}

pub fn save(
    editor_state: &GraphEditorState,
    custom_state: &CustomGraphState,
    path: PathBuf,
) -> Result<()> {
    let writer = std::io::BufWriter::new(std::fs::File::create(&path)?);
    ron::ser::to_writer_pretty(
        writer,
        &SerializedEditorState::from_state(editor_state, custom_state),
        ron::ser::PrettyConfig::default(),
    )?;
    let (bjk_graph, mapping) = graph_interop::ui_graph_to_blackjack_graph(
        &editor_state.graph,
        &custom_state.node_definitions,
    )?;
    let external_param_values =
        graph_interop::extract_graph_params(&editor_state.graph, &bjk_graph, &mapping)?;
    let positions = editor_state
        .node_positions
        .iter()
        .map(|(node_id, pos2)| (mapping[node_id], glam::Vec2::new(pos2.x, pos2.y)))
        .collect();
    blackjack_engine::graph::serialization::SerializedBjkGraph::write_to_file(
        format!("{}.bjk", path.to_str().unwrap().trim_end_matches(".blj")),
        RuntimeData {
            graph: bjk_graph,
            external_parameters: Some(external_param_values),
            positions: Some(positions),
        },
    )?;

    Ok(())
}

pub fn load(path: PathBuf) -> Result<(GraphEditorState, CustomGraphState)> {
    let reader = std::io::BufReader::new(std::fs::File::open(path)?);
    let state: SerializedEditorState = ron::de::from_reader(reader)?;
    Ok(state.into_state())
}
