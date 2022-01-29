use crate::prelude::*;
use std::path::PathBuf;

use super::{editor_state::GraphEditorState, node_finder::NodeFinder};
use crate::prelude::graph::{Graph, InputId, NodeId};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct SerializedEditorState {
    pub graph: Graph,
    pub active_node: Option<NodeId>,
    pub egui_memory: egui::Memory,
}

impl SerializedEditorState {
    pub fn from_state(editor_state: &GraphEditorState, egui_ctx: &egui::CtxRef) -> Self {
        SerializedEditorState {
            graph: editor_state.graph.clone(),
            active_node: editor_state.active_node.clone(),
            egui_memory: egui_ctx.memory().clone(),
        }
    }

    pub fn to_state(self, egui_ctx: &egui::CtxRef) -> GraphEditorState {
        let mut state = GraphEditorState::new(); 
        state.graph = self.graph;
        state.active_node = self.active_node;
        *egui_ctx.memory() = self.egui_memory;
        state
    }
}

pub fn save(editor_state: &GraphEditorState, egui_ctx: &egui::CtxRef, path: PathBuf) -> Result<()> {
    let writer = std::io::BufWriter::new(std::fs::File::create(path)?);
    ron::ser::to_writer(writer, &SerializedEditorState::from_state(editor_state, egui_ctx))?;
    Ok(())
}

pub fn load(egui_ctx: &egui::CtxRef, path: PathBuf) -> Result<GraphEditorState> {
    let reader = std::io::BufReader::new(std::fs::File::open(path)?);
    let state : SerializedEditorState = ron::de::from_reader(reader)?;
    Ok(state.to_state(egui_ctx))
}
