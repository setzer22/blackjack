use blackjack_engine::{
    graph::{BjkGraph, BjkNodeId, DependencyKind},
    graph_interpreter::BjkParameter,
    lua_engine::LuaRuntime,
};
use epaint::Vec2;
use guee::{prelude::Callback, widget::DynWidget, widget_id::IdGen};
use slotmap::SecondaryMap;

use crate::widgets::{
    node_editor_widget::{NodeEditorWidget, PanZoom},
    node_widget::{NodeWidget, PortId, PortIdKind},
};

pub struct GraphEditor {
    lua_runtime: LuaRuntime,
    pan_zoom: PanZoom,
    graph: BjkGraph,
    node_positions: SecondaryMap<BjkNodeId, Vec2>,
}

#[allow(clippy::new_without_default)]
impl GraphEditor {
    pub fn new() -> Self {
        // TODO: Hardcoded path
        let runtime = LuaRuntime::initialize_with_std("./blackjack_lua/".into())
            .expect("Lua init should not fail");
        let mut graph = BjkGraph::new();
        let mut node_positions = SecondaryMap::new();

        let node = graph
            .spawn_node("MakeBox", &runtime.node_definitions)
            .unwrap();
        node_positions.insert(node, Vec2::new(40.0, 50.0));

        let node = graph
            .spawn_node("MakeCircle", &runtime.node_definitions)
            .unwrap();
        node_positions.insert(node, Vec2::new(300.0, 150.0));

        let node = graph
            .spawn_node("BevelEdges", &runtime.node_definitions)
            .unwrap();
        node_positions.insert(node, Vec2::new(400.0, 200.0));

        Self {
            lua_runtime: runtime,
            node_positions,
            pan_zoom: PanZoom::default(),
            graph,
        }
    }

    pub fn view(&self) -> DynWidget {
        let node_widgets = self.graph.nodes.iter().map(|(node_id, node)| {
            (
                self.node_positions[node_id],
                NodeWidget::from_bjk_node(
                    node_id,
                    node,
                    Callback::from_fn(move |editor: &mut GraphEditor, new_pos| {
                        editor.node_positions[node_id] += new_pos;
                    }),
                ),
            )
        });

        let mut connections = Vec::new();
        for (node_id, node) in self.graph.nodes.iter() {
            for input in node.inputs.iter() {
                match &input.kind {
                    DependencyKind::External { .. } => {}
                    DependencyKind::Connection {
                        node: other_node_id,
                        param_name: other_param_name,
                    } => {
                        let other_node = &self.graph.nodes[*other_node_id];
                        let other_param = other_node
                            .outputs
                            .iter()
                            .find(|x| x.name == *other_param_name)
                            .expect("Other param should be there");

                        connections.push((
                            PortId {
                                param: BjkParameter::new(node_id, input.name.clone()),
                                side: PortIdKind::Input,
                                data_type: input.data_type,
                            },
                            PortId {
                                param: BjkParameter::new(*other_node_id, other_param.name.clone()),
                                side: PortIdKind::Output,
                                data_type: other_param.data_type,
                            },
                        ))
                    }
                }
            }
        }

        NodeEditorWidget::new(
            IdGen::key("node_editor"),
            node_widgets.collect(),
            connections,
            self.pan_zoom,
        )
        .on_pan_zoom_change(|editor: &mut GraphEditor, new_pan_zoom| {
            editor.pan_zoom = new_pan_zoom;
        })
        .on_connection(|editor: &mut GraphEditor, (port1, port2)| {
            let (input, output) = if port1.side == PortIdKind::Input {
                (port1.param, port2.param)
            } else {
                (port2.param, port1.param)
            };
            editor
                .graph
                .add_connection(
                    output.node_id,
                    &output.param_name,
                    input.node_id,
                    &input.param_name,
                )
                .expect("Should not fail");
        })
        .on_disconnection(|editor: &mut GraphEditor, (port1, port2)| {
            let input = if port1.side == PortIdKind::Input {
                port1.param
            } else {
                port2.param
            };
            editor
                .graph
                .remove_connection(input.node_id, &input.param_name)
                .expect("Should not fail");
        })
        .build()
    }
}
