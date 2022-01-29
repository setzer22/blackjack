use crate::{
    graph::graph_editor_egui::editor_state::{GraphEditorState, PanZoom},
    prelude::*,
};
use std::path::PathBuf;

use crate::prelude::graph::{Graph, NodeId};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct SerializedEditorState {
    pub graph: Graph,
    pub active_node: Option<NodeId>,
    pub node_positions: HashMap<NodeId, egui::Pos2>,
    pub pan_zoom: PanZoom,
}

impl SerializedEditorState {
    pub fn from_state(editor_state: &GraphEditorState) -> Self {
        SerializedEditorState {
            graph: editor_state.graph.clone(),
            active_node: editor_state.active_node,
            node_positions: editor_state.node_positions.clone(),
            pan_zoom: editor_state.pan_zoom,
        }
    }

    pub fn into_state(self) -> GraphEditorState {
        let mut state = GraphEditorState::new();
        state.graph = self.graph;
        state.active_node = self.active_node;
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
