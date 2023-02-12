use blackjack_engine::{
    graph::{BjkGraph, BjkNodeId},
    lua_engine::LuaRuntime,
};
use epaint::Vec2;
use guee::{prelude::Callback, widget::DynWidget, widget_id::IdGen};
use slotmap::SecondaryMap;

use crate::widgets::{
    node_editor_widget::{NodeEditorWidget, PanZoom},
    node_widget::{NodeWidget, PortId},
};

pub struct GraphEditor {
    lua_runtime: LuaRuntime,
    pan_zoom: PanZoom,
    graph: BjkGraph,
    node_positions: SecondaryMap<BjkNodeId, Vec2>,
}

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

        NodeEditorWidget::new(
            IdGen::key("node_editor"),
            node_widgets.collect(),
            self.pan_zoom,
        )
        .on_pan_zoom_change(|editor: &mut GraphEditor, new_pan_zoom| {
            editor.pan_zoom = new_pan_zoom;
        })
        .on_connection(|editor: &mut GraphEditor, (port1, port2)| {
            let (input_node, input_param, output_node, output_param) = match (port1, port2) {
                (
                    PortId::Input {
                        node_id: input_node,
                        param_name: input_param,
                    },
                    PortId::Output {
                        node_id: output_node,
                        param_name: output_param,
                    },
                )
                | (
                    PortId::Output {
                        node_id: output_node,
                        param_name: output_param,
                    },
                    PortId::Input {
                        node_id: input_node,
                        param_name: input_param,
                    },
                ) => (input_node, input_param, output_node, output_param),
                _ => unreachable!(),
            };
            editor
                .graph
                .add_connection(output_node, &output_param, input_node, &input_param)
                .expect("Should not fail");
        })
        .build()
    }
}
