use std::path::Path;

use blackjack_engine::graph::{
    serialization::{RuntimeData, SerializedBjkGraph, SerializedUiData},
    BjkNodeId,
};
use itertools::Itertools;
use slotmap::SecondaryMap;

use crate::graph_editor::GraphEditor;

pub fn save(path: impl AsRef<Path>, editor_state: &GraphEditor) -> anyhow::Result<()> {
    let (mut ser, mappings) = SerializedBjkGraph::from_runtime(RuntimeData {
        graph: editor_state.graph.clone(),
        external_parameters: Some(editor_state.external_parameters.clone()),
    })?;

    let node_positions = editor_state
        .node_order
        .iter()
        .map(|node_id| {
            let pos = editor_state.node_positions[*node_id];
            glam::Vec2::new(pos.x, pos.y)
        })
        .collect_vec();

    let node_order = editor_state
        .node_order
        .iter()
        .map(|node_id| mappings.get_idx(*node_id))
        .try_collect()?;

    let pan = glam::Vec2::new(editor_state.pan_zoom.pan.x, editor_state.pan_zoom.pan.y);
    let zoom = editor_state.pan_zoom.zoom;

    ser.set_ui_data(SerializedUiData {
        node_positions,
        node_order,
        pan,
        zoom,
        // TODO: Bring back Gizmos
        locked_gizmo_nodes: Vec::new(),
    });

    ser.write_to_file(path)
}

pub fn load(path: impl AsRef<Path>, editor: &mut GraphEditor) -> anyhow::Result<()> {
    let de = SerializedBjkGraph::load_from_file(&path)?;

    let (rt, ui_data, mappings) = de.into_runtime()?;

    let Some(ui_data) = ui_data else {
        anyhow::bail!(
            "The file at {} doesn't have UI information. Cannot load.",
            path.as_ref().to_string_lossy()
        );
    };

    editor.graph = rt.graph;
    editor.external_parameters = rt.external_parameters.unwrap_or_default();

    editor.node_positions = ui_data
        .node_positions
        .iter()
        .enumerate()
        .map(|(idx, v)| Ok((mappings.get_id(idx)?, epaint::vec2(v.x, v.y))))
        .collect::<anyhow::Result<SecondaryMap<BjkNodeId, epaint::Vec2>>>()?;

    editor.node_order = ui_data
        .node_order
        .iter()
        .map(|idx| mappings.get_id(*idx))
        .try_collect()?;

    editor.pan_zoom.pan = epaint::vec2(ui_data.pan.x, ui_data.pan.y);
    editor.pan_zoom.zoom = ui_data.zoom;

    Ok(())
}
