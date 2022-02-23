use crate::{
    prelude::*, prelude::graph::*
};
use std::path::PathBuf;

use egui_node_graph::PanZoom;
use serde::{Deserialize, Serialize};
use slotmap::SecondaryMap;

#[derive(Serialize, Deserialize)]
struct SerializedEditorState {
    pub graph: graph::Graph,
    pub node_order: Option<Vec<NodeId>>,
    pub active_node: Option<NodeId>,
    pub node_positions: SecondaryMap<NodeId, egui::Pos2>,
    pub pan_zoom: PanZoom,
}

impl SerializedEditorState {
    pub fn from_state(editor_state: &GraphEditorState) -> Self {
        SerializedEditorState {
            graph: editor_state.graph.clone(),
            node_order: Some(editor_state.node_order.clone()),
            active_node: editor_state.user_state.active_node,
            node_positions: editor_state.node_positions.clone(),
            pan_zoom: editor_state.pan_zoom,
        }
    }

    pub fn into_state(self) -> GraphEditorState {
        let user_state = CustomGraphState {
            run_side_effect: None,
            active_node: self.active_node,
        };

        let mut state = GraphEditorState::new(1.0, user_state);
        state.graph = self.graph;
        state.node_order = self
            .node_order
            .unwrap_or_else(|| state.graph.iter_nodes().collect());
        state.node_positions = self.node_positions;
        state.pan_zoom = self.pan_zoom;
        state
    }
}

pub fn save(editor_state: &GraphEditorState, path: PathBuf) -> Result<()> {
    let writer = std::io::BufWriter::new(std::fs::File::create(path)?);
    ron::ser::to_writer(writer, &SerializedEditorState::from_state(editor_state))?;
    Ok(())
}

pub fn load(path: PathBuf) -> Result<GraphEditorState> {
    let reader = std::io::BufReader::new(std::fs::File::open(path)?);
    let state: SerializedEditorState = ron::de::from_reader(reader)?;
    Ok(state.into_state())
}
