use std::ops::Deref;

use anyhow::bail;
use blackjack_engine::{
    graph::{
        BjkGraph, BjkNode, BjkNodeId, BlackjackValue, DataType, DependencyKind, InputDefinition,
        InputParameter, InputValueConfig,
    },
    graph_interpreter::{BjkInputParameter, ExternalParameterValues},
    lua_engine::LuaRuntime,
    prelude::selection::SelectionExpression,
};
use epaint::{ahash::HashSet, Vec2};
use guee::{
    base_widgets::drag_value::{DragValue, ScaleSelector},
    callback_accessor::CallbackAccessor,
    extension_traits::Color32Ext,
    input::MouseButton,
    prelude::*,
    widget::DynWidget,
    widget_id::IdGen,
};
use slotmap::SecondaryMap;
use winit::event::VirtualKeyCode;

use crate::{
    blackjack_theme::pallette,
    widgets::{
        node_editor_widget::{Connection, Disconnection, NodeEditorWidget, PanZoom},
        node_widget::{NodeWidget, NodeWidgetPort, NodeWidgetRow, PortId, PortIdKind},
    },
};

pub mod node_finder;
use node_finder::NodeFinder;

pub struct GraphEditor {
    pub lua_runtime: LuaRuntime,
    pub pan_zoom: PanZoom,
    // The top-left corner of the editor. In window coordinates.
    // Cached after every layout.
    pub top_left: Pos2,
    pub graph: BjkGraph,
    pub node_positions: SecondaryMap<BjkNodeId, Vec2>,
    pub node_order: Vec<BjkNodeId>,
    pub external_parameters: ExternalParameterValues,
    pub node_finder: Option<NodeFinder>,
    pub cba: CallbackAccessor<Self>,
}

#[allow(clippy::new_without_default)]
impl GraphEditor {
    pub fn new(cba: CallbackAccessor<Self>) -> Self {
        Self {
            external_parameters: ExternalParameterValues::default(),
            // TODO: Hardcoded path
            lua_runtime: LuaRuntime::initialize_with_std("./blackjack_lua/".into())
                .expect("Lua init should not fail"),
            node_positions: SecondaryMap::new(),
            node_order: Vec::new(),
            top_left: Pos2::ZERO,
            pan_zoom: PanZoom::default(),
            node_finder: None,
            graph: BjkGraph::new(),
            cba,
        }
    }

    pub fn spawn_node(&mut self, op_name: &str) {
        let node_finder_pos = self.node_finder.as_ref().expect("Node finder").position;
        let spawned_node_pos =
            NodeEditorWidget::cursor_transform(self.pan_zoom, self.top_left.to_vec2())
                .transform_point(node_finder_pos);
        let new = self
            .graph
            .spawn_node(op_name, &self.lua_runtime.node_definitions)
            .expect("Spawn node should not fail");
        self.node_positions.insert(new, spawned_node_pos.to_vec2());
        self.node_order.push(new);
    }

    pub fn remove_node(&mut self, node: BjkNodeId) {
        if self.graph.default_node == Some(node) {
            // Heuristic: When removing the active node, look for the previous
            // node connected to the one that got deleted, and activate it.
            let old_node = &self.graph.nodes[node];
            for input in &old_node.inputs {
                match &input.kind {
                    DependencyKind::External { .. } => (),
                    DependencyKind::Connection { node, .. } => {
                        self.graph.default_node = Some(*node);
                        break;
                    }
                }
            }
        }
        self.graph.remove_node(node);
        self.node_positions.remove(node);
        self.node_order.retain(|x| *x != node);
    }

    pub fn view(&self) -> DynWidget {
        // Ensure that the node_order and the graph are always aligned.
        debug_assert_eq!(
            self.graph
                .nodes
                .iter()
                .map(|(id, _)| id)
                .collect::<HashSet<_>>()
                .difference(&self.node_order.iter().copied().collect())
                .count(),
            0,
            "Inconsistency between node_order and graph",
        );

        // Ensure that the node_positions and the graph are always aligned.
        debug_assert_eq!(
            self.graph
                .nodes
                .iter()
                .map(|(id, _)| id)
                .collect::<HashSet<_>>()
                .difference(&self.node_positions.iter().map(|(k, _v)| k).collect())
                .count(),
            0,
            "Inconsistency between node_positions and graph",
        );

        let node_widgets = self.node_order.iter().copied().map(|node_id| {
            let node = &self.graph.nodes[node_id];
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
                                node_id,
                                param_name: input.name.clone(),
                                side: PortIdKind::Input,
                                data_type: input.data_type,
                            },
                            PortId {
                                node_id: *other_node_id,
                                param_name: other_param.name.clone(),
                                side: PortIdKind::Output,
                                data_type: other_param.data_type,
                            },
                        ))
                    }
                }
            }
        }

        let node_editor = NodeEditorWidget::new(
            IdGen::key("node_editor"),
            node_widgets.collect(),
            connections,
            self.pan_zoom,
        )
        .on_pan_zoom_change(self.cba.callback(|editor, new_pan_zoom| {
            editor.pan_zoom = new_pan_zoom;
        }))
        .on_connection(self.cba.callback(|editor, conn: Connection| {
            editor
                .graph
                .add_connection(conn.output.0, &conn.output.1, conn.input.0, &conn.input.1)
                .expect("Should not fail");
        }))
        .on_disconnection(self.cba.callback(|editor, disc: Disconnection| {
            editor
                .graph
                .remove_connection(disc.input.0, &disc.input.1)
                .expect("Should not fail");
        }))
        .on_node_raised(self.cba.callback(|editor, node_id| {
            // When the node is deleted, a "raised" event will be emitted but we
            // don't want to handle it then.
            if editor.node_order.contains(&node_id) {
                editor.node_order.retain(|x| *x != node_id);
                editor.node_order.push(node_id);
            }
        }))
        .build();

        let mut stack = vec![(Vec2::new(0.0, 0.0), node_editor)];

        if let Some(node_finder) = self.node_finder.as_ref() {
            stack.push((
                node_finder.position.to_vec2(),
                node_finder.view(&self.lua_runtime.node_definitions),
            ));
        }

        let stack = StackContainer::new(IdGen::key("stack"), stack).build();

        // We use this container to detect unhandled right click events for the
        // graph editor and spawn the node finder at that position.
        let cba_cpy = self.cba.clone();
        let on_spawn_finder_cb = self.cba.callback(move |editor, spawn_pos: Pos2| {
            editor.node_finder = Some(NodeFinder::new(cba_cpy, spawn_pos));
        });
        let store_layout_cb = self.cba.callback(move |editor, top_left: Pos2| {
            editor.top_left = top_left;
        });
        let on_dismiss_node_finder_cb = self.cba.callback(move |editor, _: ()| {
            editor.node_finder = None;
        });
        TinkerContainer::new(stack)
            .post_layout(|ctx, layout| {
                ctx.dispatch_callback(store_layout_cb, layout.bounds.left_top());
            })
            .post_event(move |ctx, layout, cursor_position, events| {
                let cursor_in_finder = layout
                    .children
                    .get(1)
                    .map(|x| x.bounds.contains(cursor_position))
                    .unwrap_or(false);

                if layout.bounds.contains(cursor_position)
                    && ctx
                        .input_state
                        .mouse
                        .button_state
                        .is_clicked(MouseButton::Secondary)
                {
                    ctx.dispatch_callback(
                        on_spawn_finder_cb,
                        (cursor_position - layout.bounds.left_top()).to_pos2(),
                    );
                    EventStatus::Consumed
                } else if (!cursor_in_finder
                    && ctx
                        .input_state
                        .mouse
                        .button_state
                        .is_clicked(MouseButton::Primary))
                    || events
                        .iter()
                        .any(|ev| matches!(ev, Event::KeyPressed(VirtualKeyCode::Escape)))
                {
                    ctx.dispatch_callback(on_dismiss_node_finder_cb, ());
                    EventStatus::Consumed
                } else {
                    EventStatus::Ignored
                }
            })
            .build()
    }

    pub fn make_node_widget(&self, node_id: BjkNodeId, node: &BjkNode) -> NodeWidget {
        let mut rows = Vec::new();

        /// Returns the color of a data type
        fn data_type_color(data_type: DataType) -> Color32 {
            match data_type {
                DataType::Mesh => color!("#b43e3e"),
                DataType::HeightMap => color!("#33673b"),
                DataType::Vector => color!("#1A535C"),
                DataType::Scalar => color!("#4ecdc4"),
                DataType::Selection => color!("#f7fff7"),
                DataType::String => color!("#ffe66d"),
            }
        }

        for input in &node.inputs {
            rows.push((
                PortId {
                    node_id,
                    param_name: input.name.clone(),
                    side: PortIdKind::Input,
                    data_type: input.data_type,
                },
                NodeWidgetRow {
                    input_port: Some(NodeWidgetPort {
                        color: data_type_color(input.data_type),
                        // Set later, by the node editor, which does the event
                        // checking for ports.
                        hovered: false,
                        data_type: input.data_type,
                    }),
                    contents: self.make_in_parameter_widget(node_id, input),
                    output_port: None,
                    align: Align::Start,
                },
            ));
        }
        for output in &node.outputs {
            rows.push((
                PortId {
                    node_id,
                    param_name: output.name.clone(),
                    side: PortIdKind::Output,
                    data_type: output.data_type,
                },
                NodeWidgetRow {
                    input_port: None,
                    contents: Text::new(output.name.clone()).build(),
                    output_port: Some(NodeWidgetPort {
                        color: data_type_color(output.data_type),
                        hovered: false, // See above
                        data_type: output.data_type,
                    }),
                    align: Align::End,
                },
            ));
        }

        let set_active_cb = self.cba.callback(move |editor, _| {
            editor.graph.default_node = Some(node_id);
        });
        let bottom_ui = if self.graph.default_node == Some(node_id) {
            Button::with_label("👁 Active")
                .padding(Vec2::new(5.0, 3.0))
                .on_click(set_active_cb)
                .style_override(ButtonStyle::with_base_colors(
                    pallette().accent.lighten(0.8),
                    Stroke::NONE,
                    1.1,
                    1.3,
                ))
                .build()
        } else {
            Button::with_label("👁 Set Active")
                .padding(Vec2::new(5.0, 3.0))
                .on_click(set_active_cb)
                .build()
        };

        let node_title = self
            .lua_runtime
            .node_definitions
            .node_def(&node.op_name)
            .map(|x| x.label.clone())
            .unwrap_or_else(|| node.op_name.clone());
        NodeWidget {
            id: IdGen::key(node_id),
            node_id,
            titlebar_left: MarginContainer::new(
                IdGen::key("margin_l"),
                Text::new(node_title).build(),
            )
            .margin(Vec2::new(10.0, 10.0))
            .build(),
            titlebar_right: MarginContainer::new(
                IdGen::key("margin_r"),
                Button::with_label("x")
                    .padding(Vec2::ZERO)
                    .on_click(self.cba.callback(move |editor, _| {
                        editor.remove_node(node_id);
                    }))
                    .build(),
            )
            .margin(Vec2::new(10.0, 10.0))
            .build(),
            bottom_ui,
            rows,
            v_separation: 4.0,
            h_separation: 12.0,
            extra_v_separation: 3.0,
            on_node_dragged: Some(self.cba.callback(move |editor, delta| {
                editor.node_positions[node_id] += delta / editor.pan_zoom.zoom;
            })),
        }
    }

    pub fn make_in_parameter_widget(
        &self,
        node_id: BjkNodeId,
        input: &InputParameter,
    ) -> DynWidget {
        let name_label = Text::new(input.name.clone()).build();
        let op_name = &self.graph.nodes[node_id].op_name;
        let param = BjkInputParameter::new(node_id, input.name.clone());
        match &input.kind {
            DependencyKind::External { promoted: _ } => match input.data_type {
                DataType::Vector => self.make_vector_param_widget(&param, op_name),
                DataType::Scalar => self.make_scalar_param_widget(&param, op_name),
                DataType::Selection => self.make_selection_param_widget(&param, op_name),
                DataType::Mesh => name_label,
                DataType::String => name_label,
                DataType::HeightMap => name_label,
            },
            DependencyKind::Connection {
                node: _,
                param_name: _,
            } => name_label,
        }
    }

    pub fn get_current_param_value(
        &self,
        param: &BjkInputParameter,
        op_name: &str,
    ) -> anyhow::Result<(BlackjackValue, impl Deref<Target = InputDefinition> + '_)> {
        if let Some(input_def) = self
            .lua_runtime
            .node_definitions
            .input_def(op_name, &param.param_name)
        {
            if let Some(existing) = self.external_parameters.0.get(param) {
                Ok((existing.clone(), input_def))
            } else {
                Ok((input_def.default_value(), input_def))
            }
        } else {
            bail!("Not found in node definitions")
        }
    }

    pub fn make_scalar_param_widget(&self, param: &BjkInputParameter, op_name: &str) -> DynWidget {
        if let Ok((BlackjackValue::Scalar(current), input_def)) =
            self.get_current_param_value(param, op_name)
        {
            let param_cpy = param.clone();

            let InputValueConfig::Scalar {
                default: _,
                min,
                max,
                soft_min,
                soft_max,
                num_decimals
            } = &input_def.config else {
                unreachable!("Wrong scalar config type")
            };

            BoxContainer::horizontal(
                IdGen::key(param),
                vec![
                    Text::new(param.param_name.clone()).build(),
                    DragValue::new(IdGen::key((param, "value")), current as f64)
                        .on_changed(self.cba.callback(|editor: &mut GraphEditor, new| {
                            editor
                                .external_parameters
                                .0
                                .insert(param_cpy, BlackjackValue::Scalar(new as f32));
                        }))
                        .speed(1.0)
                        .scale_selector(if num_decimals == &Some(0) {
                            Some(ScaleSelector::int_3vals())
                        } else {
                            Some(ScaleSelector::float_7vals())
                        })
                        .num_decimals(num_decimals.unwrap_or(4))
                        .default_scale_selector_index(if num_decimals == &Some(0) {
                            // Select the last row, corresponding to the value
                            // 1, for integer sliders
                            Some(2)
                        } else {
                            None
                        })
                        .soft_range(
                            soft_min.unwrap_or(min.unwrap_or(-f32::INFINITY)).into()
                                ..=soft_max.unwrap_or(max.unwrap_or(f32::INFINITY)).into(),
                        )
                        .hard_range(
                            min.unwrap_or(-f32::INFINITY).into()
                                ..=max.unwrap_or(f32::INFINITY).into(),
                        )
                        .layout_hints(LayoutHints::fill_horizontal())
                        .build(),
                ],
            )
            .layout_hints(LayoutHints::fill_horizontal())
            .separation(10.0)
            .build()
        } else {
            Text::new("<error>".into()).build()
        }
    }

    pub fn make_vector_param_widget(&self, param: &BjkInputParameter, op_name: &str) -> DynWidget {
        if let Ok((BlackjackValue::Vector(current), _input_def)) =
            self.get_current_param_value(param, op_name)
        {
            macro_rules! component_drag_val {
                ($field:ident) => {{
                    let param_cpy = param.clone();
                    let mut current_cpy = current;
                    DragValue::new(
                        IdGen::key((param, stringify!($field))),
                        current.$field as f64,
                    )
                    .on_changed(self.cba.callback(move |editor: &mut GraphEditor, new| {
                        current_cpy.$field = new as f32;
                        editor
                            .external_parameters
                            .0
                            .insert(param_cpy, BlackjackValue::Vector(current_cpy));
                    }))
                    .scale_selector(Some(ScaleSelector::float_7vals()))
                    .speed(1.0)
                    .layout_hints(LayoutHints::fill_horizontal())
                    .build()
                }};
            }

            BoxContainer::vertical(
                IdGen::key(param),
                vec![
                    Text::new(param.param_name.clone()).build(),
                    BoxContainer::horizontal(
                        IdGen::key("h"),
                        vec![
                            component_drag_val!(x),
                            component_drag_val!(y),
                            component_drag_val!(z),
                        ],
                    )
                    .layout_hints(LayoutHints::fill_horizontal())
                    .separation(0.0)
                    .build(),
                ],
            )
            .build()
        } else {
            Text::new("<error>".into()).build()
        }
    }

    pub fn make_selection_param_widget(
        &self,
        param: &BjkInputParameter,
        op_name: &str,
    ) -> DynWidget {
        if let Ok((BlackjackValue::Selection(current_str, _), input_def)) =
            self.get_current_param_value(param, op_name)
        {
            let param_cpy = param.clone();

            let InputValueConfig::Selection {
                default_selection: _,
            } = &input_def.config else {
                unreachable!("Wrong selection config type")
            };

            TextEdit::new(IdGen::key((param, "text_edit")), current_str)
                .on_changed(self.cba.callback(|editor, new_str: String| {
                    let parsed = SelectionExpression::parse(&new_str).ok();
                    editor
                        .external_parameters
                        .0
                        .insert(param_cpy, BlackjackValue::Selection(new_str, parsed));
                }))
                .layout_hints(LayoutHints::fill_horizontal())
                .build()
        } else {
            Text::new("<error>".into()).build()
        }
    }
}

// WIP:
// - [x] Use human-friendly labels in nodes.
// - [x] Node widget alignment improvements.
// - [x] Use the color for data types.
// - [x] Top menubar
// - [x] Save / load system
// - [ ] Display errors in a console
