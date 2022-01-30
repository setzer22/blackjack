use self::{graph_node_ui::*, node_finder::NodeFinder};
use crate::prelude::*;
use editor_state::GraphEditorState;
use egui::*;

use super::graph_types::AnyParameterId;

pub mod editor_state;

pub mod graph_node_ui;

pub mod node_finder;

pub fn draw_graph_editor(ctx: &CtxRef, state: &mut GraphEditorState) {
    let mouse = &ctx.input().pointer;
    let cursor_pos = mouse.hover_pos().unwrap_or(Pos2::ZERO);

    // Gets filled with the port locations as nodes are drawn
    let mut port_locations = PortLocations::new();

    // The responses returned from node drawing have side effects that are best
    // executed at the end of this function.
    let mut delayed_responses: Vec<DrawGraphNodeResponse> = vec![];

    // Used to detect when the background was clicked, to dismiss certain states
    let mut click_on_background = false;

    CentralPanel::default().show(ctx, |ui| {
        /* Draw nodes */
        let nodes = state.graph.iter_nodes().collect::<Vec<_>>(); // avoid borrow checker
        for node_id in nodes {
            let responses = GraphNodeWidget {
                position: state.node_positions.get_mut(&node_id).unwrap(),
                graph: &mut state.graph,
                port_locations: &mut port_locations,
                node_id,
                ongoing_drag: state.connection_in_progress,
                active: state
                    .active_node
                    .map(|active| active == node_id)
                    .unwrap_or(false),
                selected: state
                    .selected_node
                    .map(|selected| selected == node_id)
                    .unwrap_or(false),
                pan: state.pan_zoom.pan,
            }
            .show(ui);

            // Actions executed later
            delayed_responses.extend(responses);
        }

        let r = ui.allocate_rect(ui.min_rect(), Sense::click());
        if r.clicked() {
            click_on_background = true;
        }
    });

    /* Draw the node finder, if open */
    let mut should_close_node_finder = false;
    if let Some(ref mut node_finder) = state.node_finder {
        let mut node_finder_area = Area::new("node_finder");
        if let Some(pos) = node_finder.position {
            node_finder_area = node_finder_area.current_pos(pos);
        }
        node_finder_area.show(ctx, |ui| {
            if let Some(node_archetype) = node_finder.show(ui) {
                let new_node = state.graph.add_node(node_archetype.to_descriptor());
                state
                    .node_positions
                    .insert(new_node, cursor_pos - state.pan_zoom.pan);
                should_close_node_finder = true;
            }
        });
    }
    if should_close_node_finder {
        state.node_finder = None;
    }

    /* Draw connections */
    let connection_stroke = egui::Stroke {
        width: 5.0,
        color: color_from_hex("#efefef").unwrap(),
    };

    if let Some((_, ref locator)) = state.connection_in_progress {
        let painter = ctx.layer_painter(LayerId::background());
        let start_pos = port_locations[locator];
        painter.line_segment([start_pos, cursor_pos], connection_stroke)
    }

    for (input, output) in state.graph.iter_connections() {
        let painter = ctx.layer_painter(LayerId::background());
        let src_pos = port_locations[&AnyParameterId::Output(output)];
        let dst_pos = port_locations[&AnyParameterId::Input(input)];
        painter.line_segment([src_pos, dst_pos], connection_stroke);
    }

    /* Handle responses from drawing nodes */

    for response in delayed_responses {
        match response {
            DrawGraphNodeResponse::ConnectEventStarted(node_id, port) => {
                state.connection_in_progress = Some((node_id, port));
            }
            DrawGraphNodeResponse::ConnectEventEnded(locator) => {
                let in_out = match (
                    state
                        .connection_in_progress
                        .map(|(_node, param)| param)
                        .take()
                        .expect("Cannot end drag without in-progress connection."),
                    locator,
                ) {
                    (AnyParameterId::Input(input), AnyParameterId::Output(output))
                    | (AnyParameterId::Output(output), AnyParameterId::Input(input)) => {
                        Some((input, output))
                    }
                    _ => None,
                };

                if let Some((input, output)) = in_out {
                    state.graph.add_connection(output, input)
                }
            }
            DrawGraphNodeResponse::SetActiveNode(node_id) => {
                state.active_node = Some(node_id);
            }
            DrawGraphNodeResponse::SelectNode(node_id) => {
                state.selected_node = Some(node_id);
            }
            DrawGraphNodeResponse::ClearActiveNode => {
                state.active_node = None;
            }
            DrawGraphNodeResponse::RunNodeSideEffect(node_id) => {
                state.run_side_effect = Some(node_id);
            }
            DrawGraphNodeResponse::DeleteNode(node_id) => {
                state.graph.remove_node(node_id);
                state.node_positions.remove(&node_id);
                // Make sure to not leave references to old nodes hanging
                if state.active_node.map(|x| x == node_id).unwrap_or(false) {
                    state.active_node = None;
                }
                if state.selected_node.map(|x| x == node_id).unwrap_or(false) {
                    state.selected_node = None;
                }
                if state.run_side_effect.map(|x| x == node_id).unwrap_or(false) {
                    state.run_side_effect = None;
                }
            }
            DrawGraphNodeResponse::DisconnectEvent(input_id) => {
                let corresp_output = state
                    .graph
                    .connection(input_id)
                    .expect("Connection data should be valid");
                let other_node = state.graph.get_input(input_id).node();
                state.graph.remove_connection(input_id);
                state.connection_in_progress =
                    Some((other_node, AnyParameterId::Output(corresp_output)));
            }
        }
    }

    /* Mouse input handling */

    if mouse.any_released() && state.connection_in_progress.is_some() {
        state.connection_in_progress = None;
    }

    if mouse.button_down(PointerButton::Secondary) {
        state.node_finder = Some(NodeFinder::new_at(cursor_pos));
    }
    if ctx.input().key_pressed(Key::Escape) {
        state.node_finder = None;
    }

    if ctx.input().pointer.middle_down() {
        state.pan_zoom.pan += ctx.input().pointer.delta();
    }

    if click_on_background {
        state.selected_node = None;
        state.node_finder = None;
    }
}
