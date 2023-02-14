use anyhow::bail;
use blackjack_engine::{
    graph::{
        BjkGraph, BjkNode, BjkNodeId, BlackjackValue, DataType, DependencyKind, InputParameter,
    },
    graph_interpreter::{BjkParameter, ExternalParameterValues},
    lua_engine::LuaRuntime,
};
use epaint::Vec2;
use guee::{prelude::*, widget::DynWidget, widget_id::IdGen};
use slotmap::SecondaryMap;

use crate::widgets::{
    node_editor_widget::{NodeEditorWidget, PanZoom},
    node_widget::{NodeWidget, NodeWidgetPort, NodeWidgetRow, PortId, PortIdKind},
};

pub struct GraphEditor {
    lua_runtime: LuaRuntime,
    pan_zoom: PanZoom,
    graph: BjkGraph,
    node_positions: SecondaryMap<BjkNodeId, Vec2>,
    external_parameters: ExternalParameterValues,
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
            external_parameters: ExternalParameterValues::default(),
            lua_runtime: runtime,
            node_positions,
            pan_zoom: PanZoom::default(),
            graph,
        }
    }

    pub fn make_in_parameter_widget(
        &self,
        node_id: BjkNodeId,
        input: &InputParameter,
    ) -> DynWidget {
        let name_label = Text::new(input.name.clone()).build();
        let op_name = &self.graph.nodes[node_id].op_name;
        match &input.kind {
            DependencyKind::External { promoted } => match input.data_type {
                DataType::Vector => name_label,
                DataType::Scalar => self.make_scalar_param_widget(
                    &BjkParameter::new(node_id, input.name.clone()),
                    op_name,
                ),
                DataType::Selection => name_label,
                DataType::Mesh => name_label,
                DataType::String => name_label,
                DataType::HeightMap => name_label,
            },
            DependencyKind::Connection { node, param_name } => name_label,
        }
    }

    pub fn make_node_widget(&self, node_id: BjkNodeId, node: &BjkNode) -> NodeWidget {
        let mut rows = Vec::new();
        for input in &node.inputs {
            rows.push((
                BjkParameter::new(node_id, input.name.clone()),
                NodeWidgetRow {
                    input_port: Some(NodeWidgetPort {
                        color: color!("#ff0000"),
                        // Set later, by the node editor, which does the event
                        // checking for ports.
                        hovered: false,
                        data_type: input.data_type,
                    }),
                    contents: self.make_in_parameter_widget(node_id, input),
                    output_port: None,
                },
            ));
        }
        for output in &node.outputs {
            rows.push((
                BjkParameter::new(node_id, output.name.clone()),
                NodeWidgetRow {
                    input_port: None,
                    contents: Text::new(output.name.clone()).build(),
                    output_port: Some(NodeWidgetPort {
                        color: color!("#00ff00"),
                        hovered: false, // See above
                        data_type: output.data_type,
                    }),
                },
            ));
        }

        NodeWidget {
            id: IdGen::key(node_id),
            node_id,
            titlebar_left: MarginContainer::new(
                IdGen::key("margin_l"),
                Text::new(node.op_name.clone()).build(),
            )
            .margin(Vec2::new(10.0, 10.0))
            .build(),
            titlebar_right: MarginContainer::new(
                IdGen::key("margin_r"),
                Button::with_label("x").padding(Vec2::ZERO).build(),
            )
            .margin(Vec2::new(10.0, 10.0))
            .build(),
            bottom_ui: Button::with_label("Set Active")
                .padding(Vec2::new(5.0, 3.0))
                .build(),
            rows,
            v_separation: 4.0,
            h_separation: 12.0,
            extra_v_separation: 3.0,
            on_node_dragged: Some(Callback::from_fn(move |editor: &mut GraphEditor, delta| {
                editor.node_positions[node_id] += delta / editor.pan_zoom.zoom;
            })),
        }
    }

    pub fn view(&self) -> DynWidget {
        let node_widgets = self.graph.nodes.iter().map(|(node_id, node)| {
            (
                self.node_positions[node_id],
                self.make_node_widget(node_id, node),
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
        .on_connection(|editor: &mut GraphEditor, conn| {
            editor
                .graph
                .add_connection(
                    conn.output.node_id,
                    &conn.output.param_name,
                    conn.input.node_id,
                    &conn.input.param_name,
                )
                .expect("Should not fail");
        })
        .on_disconnection(|editor: &mut GraphEditor, disc| {
            editor
                .graph
                .remove_connection(disc.input.node_id, &disc.input.param_name)
                .expect("Should not fail");
        })
        .build()
    }

    pub fn get_current_param_value(
        &self,
        param: &BjkParameter,
        op_name: &str,
    ) -> anyhow::Result<BlackjackValue> {
        if let Some(input_def) = self
            .lua_runtime
            .node_definitions
            .input_def(op_name, &param.param_name)
        {
            if let Some(existing) = self.external_parameters.0.get(param) {
                Ok(existing.clone())
            } else {
                Ok(input_def.default_value())
            }
        } else {
            bail!("Not found in node definitions")
        }
    }

    pub fn make_scalar_param_widget(&self, param: &BjkParameter, op_name: &str) -> DynWidget {
        if let Ok(BlackjackValue::Scalar(current)) = self.get_current_param_value(param, op_name) {
            let param_cpy = param.clone();
            // WIP: This is "working", but not really. We can't just reuse
            // TextEdit to convert to string and parse back. We need a widget
            // like "DragValue" that will handle this properly.
            TextEdit::new(IdGen::key(param), format!("{current:.4}"))
                .on_changed(|editor: &mut GraphEditor, new_contents: String| {
                    if let Ok(f) = new_contents.parse::<f32>() {
                        editor
                            .external_parameters
                            .0
                            .insert(param_cpy, BlackjackValue::Scalar(f));
                    }
                })
                .build()
        } else {
            Text::new("<error>".into()).build()
        }
    }
}
