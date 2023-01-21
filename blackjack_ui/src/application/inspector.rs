// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::prelude::{
    graph::{CustomGraphState, Graph},
    *,
};
use blackjack_engine::{
    lua_engine::RenderableThing,
    prelude::{selection::SelectionExpression, ChannelKeyType, ChannelValueType, HalfEdgeMesh},
};
use egui::*;
use egui_node_graph::{InputId, NodeId, WidgetValueTrait};
use slotmap::Key;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InspectorTab {
    Properties,
    Spreadsheet,
    Debug,
}

pub struct InspectorTabs {
    current_view: InspectorTab,
    properties: PropertiesTab,
    spreadsheet: SpreadsheetTab,
    debug: DebugTab,
}

impl InspectorTabs {
    pub fn new() -> Self {
        Self {
            current_view: InspectorTab::Properties,
            properties: PropertiesTab {
                new_promoted_popup: None,
            },
            spreadsheet: SpreadsheetTab {
                current_view: SpreadsheetViews::Vertices,
            },
            debug: DebugTab {
                mesh_element: ChannelKeyType::VertexId,
                v_query: "".into(),
                f_query: "".into(),
                h_query: "".into(),
            },
        }
    }
}

impl Default for InspectorTabs {
    fn default() -> Self {
        Self::new()
    }
}

pub struct NewPromotedPopup {
    name: String,
    param: InputId,
}

pub struct PropertiesTab {
    new_promoted_popup: Option<NewPromotedPopup>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SpreadsheetViews {
    Vertices,
    Halfedges,
    Faces,
}

pub struct SpreadsheetTab {
    pub current_view: SpreadsheetViews,
}

pub struct DebugTab {
    pub mesh_element: ChannelKeyType,
    pub v_query: String,
    pub f_query: String,
    pub h_query: String,
}

impl InspectorTabs {
    pub fn ui(
        &mut self,
        ui: &mut Ui,
        renderable_thing: Option<&RenderableThing>,
        editor_state: &mut graph::GraphEditorState,
        custom_state: &mut graph::CustomGraphState,
    ) {
        match renderable_thing {
            Some(RenderableThing::HalfEdgeMesh(mesh)) => {
                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut self.current_view,
                        InspectorTab::Properties,
                        "Inspector",
                    );
                    ui.selectable_value(
                        &mut self.current_view,
                        InspectorTab::Spreadsheet,
                        "Spreadsheet",
                    );
                    ui.selectable_value(&mut self.current_view, InspectorTab::Debug, "Debug");
                });
                ui.separator();

                match self.current_view {
                    InspectorTab::Properties => self.properties.ui(ui, editor_state, custom_state),
                    InspectorTab::Spreadsheet => self.spreadsheet.ui(ui, Some(mesh)),
                    InspectorTab::Debug => self.debug.ui(ui, Some(mesh)),
                }
            }
            Some(RenderableThing::HeightMap(_)) => {
                // TODO: @Heightmap
            }
            None => { /**/ }
        }
    }
}

pub fn tiny_checkbox(ui: &mut Ui, value: &mut bool) {
    let mut child_ui = ui.child_ui(ui.available_rect_before_wrap(), *ui.layout());
    child_ui.spacing_mut().icon_spacing = 0.0;
    child_ui.spacing_mut().interact_size = egui::vec2(16.0, 16.0);
    child_ui.checkbox(value, "");
    ui.add_space(24.0);
}

impl PropertiesTab {
    fn maybe_show_new_promoted_modal(
        &mut self,
        ctx: &egui::Context,
        editor_state: &mut graph::GraphEditorState,
        custom_state: &mut graph::CustomGraphState,
    ) {
        let mut should_clear_popup = false;
        if let Some(popup) = &mut self.new_promoted_popup {
            Window::new("new_promoted_paramater_modal")
                .anchor(Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label("Name of the parameter");
                    ui.text_edit_singleline(&mut popup.name);
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            should_clear_popup = true;
                        } else if ui.button("Ok").clicked() {
                            should_clear_popup = true;
                            // Might've been removed by now. If so, simply ignore.
                            if let Some(input) = editor_state.graph.inputs.get_mut(popup.param) {
                                custom_state
                                    .promoted_params
                                    .insert(input.id, popup.name.take());
                            }
                        }
                    });
                });
        }
        if should_clear_popup {
            self.new_promoted_popup = None;
        }
    }

    fn ui(
        &mut self,
        ui: &mut Ui,
        editor_state: &mut graph::GraphEditorState,
        custom_state: &mut CustomGraphState,
    ) {
        self.maybe_show_new_promoted_modal(ui.ctx(), editor_state, custom_state);

        let graph = &mut editor_state.graph;
        if !editor_state.selected_nodes.is_empty() {
            Frame::default()
                .inner_margin(egui::vec2(5.0, 0.0))
                .show(ui, |ui| {
                    ScrollArea::both().show(ui, |ui| {
                        for node in editor_state.selected_nodes.iter_cpy().sorted() {
                            let mut ch_ui = ui.child_ui_with_id_source(
                                ui.available_rect_before_wrap(),
                                *ui.layout(),
                                node,
                            );
                            self.draw_params_for_node(&mut ch_ui, graph, custom_state, node);
                            ui.allocate_rect(ch_ui.min_rect(), Sense::hover());
                        }
                    });
                });
        } else {
            ui.label("No node selected.");
        }
    }

    fn draw_params_for_node(
        &mut self,
        ui: &mut egui::Ui,
        graph: &mut Graph,
        custom_state: &mut CustomGraphState,
        node_id: NodeId,
    ) {
        let node = &graph[node_id];
        let inputs = node.inputs.clone();
        ui.label(format!("{} ({:?})", node.label, node.id.data()));
        ui.vertical(|ui| {
            for (param_name, param) in inputs {
                if graph.connection(param).is_some() {
                    ui.label(param_name);
                } else {
                    ui.horizontal(|ui| {
                        tiny_checkbox(ui, &mut graph[param].shown_inline);
                        let mut defer_remove_promoted_param = None;
                        if let Some(ref promoted_name) = custom_state.promoted_params.get(&param) {
                            if ui
                                .button("❌")
                                .on_hover_text(format!(
                                    "Promoted as '{promoted_name}'. Click to undo."
                                ))
                                .clicked()
                            {
                                defer_remove_promoted_param = Some(param);
                            };
                        } else if ui
                            .button("⤴")
                            .on_hover_text(
                                "Click to promote parameter. \
                            Promoted parameters will be tweakable in exported jacks.",
                            )
                            .clicked()
                        {
                            self.new_promoted_popup = Some(NewPromotedPopup {
                                name: String::new(),
                                param,
                            })
                        }

                        if let Some(id) = defer_remove_promoted_param {
                            custom_state.promoted_params.remove(&id);
                        }

                        let mut value = std::mem::take(&mut graph[param].value);
                        value.value_widget(
                            &param_name,
                            node_id,
                            ui,
                            custom_state,
                            &graph[node_id].user_data,
                        );
                        graph[param].value = value;
                    });
                }
            }
        });
        ui.separator();
    }
}
impl SpreadsheetTab {
    fn ui(&mut self, ui: &mut Ui, mesh: Option<&HalfEdgeMesh>) {
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut self.current_view,
                SpreadsheetViews::Vertices,
                "Vertices",
            );
            ui.selectable_value(&mut self.current_view, SpreadsheetViews::Faces, "Faces");
            ui.selectable_value(
                &mut self.current_view,
                SpreadsheetViews::Halfedges,
                "Half edges",
            );
        });

        if let Some(mesh) = mesh {
            let channel_introspect = mesh.channels.introspect(mesh.gen_introspect_fn());

            let scroll_area = ScrollArea::both().auto_shrink([false, false]);
            scroll_area.show(ui, |ui| {
                let mut columns = vec![];
                let kt = match self.current_view {
                    SpreadsheetViews::Vertices => ChannelKeyType::VertexId,
                    SpreadsheetViews::Halfedges => ChannelKeyType::HalfEdgeId,
                    SpreadsheetViews::Faces => ChannelKeyType::FaceId,
                };
                for vt in [
                    ChannelValueType::Vec3,
                    ChannelValueType::f32,
                    ChannelValueType::bool,
                ] {
                    if let Some(ch) = channel_introspect.get(&(kt, vt)) {
                        for (ch_name, ch_contents) in ch.iter() {
                            columns.push((ch_name, ch_contents));
                        }
                    }
                }

                Grid::new("vertex-spreadsheet")
                    .striped(true)
                    .num_columns(columns.len())
                    .show(ui, |ui| {
                        ui.label(" ");
                        for c in &columns {
                            ui.label(c.0);
                        }
                        ui.end_row();

                        if !columns.is_empty() {
                            for i in 0..columns[0].1.len() {
                                ui.label(i.to_string());
                                for c in &columns {
                                    ui.monospace(c.1[i].clone() + " |");
                                }
                                ui.end_row();
                            }
                        } else {
                            let count = match self.current_view {
                                SpreadsheetViews::Vertices => {
                                    mesh.read_connectivity().num_vertices()
                                }
                                SpreadsheetViews::Halfedges => {
                                    mesh.read_connectivity().num_halfedges()
                                }
                                SpreadsheetViews::Faces => mesh.read_connectivity().num_faces(),
                            };
                            for i in 0..count {
                                ui.label(i.to_string());
                                ui.end_row();
                            }
                        }
                    })
            });
        }
    }
}
impl DebugTab {
    fn ui(&mut self, ui: &mut Ui, mesh: Option<&HalfEdgeMesh>) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.mesh_element, ChannelKeyType::VertexId, "Vertex");
            ui.selectable_value(&mut self.mesh_element, ChannelKeyType::FaceId, "Face");
            ui.selectable_value(
                &mut self.mesh_element,
                ChannelKeyType::HalfEdgeId,
                "Halfedge",
            );
        });

        match self.mesh_element {
            ChannelKeyType::VertexId => {
                ui.text_edit_singleline(&mut self.v_query);
            }
            ChannelKeyType::FaceId => {
                ui.text_edit_singleline(&mut self.f_query);
            }
            ChannelKeyType::HalfEdgeId => {
                ui.text_edit_singleline(&mut self.h_query);
            }
        }

        let err_label = |ui: &mut egui::Ui, err: anyhow::Error| {
            ui.label(RichText::new(err.to_string()).color(Color32::RED));
        };

        if let Some(mesh) = mesh {
            ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| match self.mesh_element {
                    ChannelKeyType::VertexId => match SelectionExpression::parse(&self.v_query) {
                        Err(err) => err_label(ui, err),
                        Ok(expr) => {
                            let conn = mesh.read_connectivity();
                            let v_mapping = conn.vertex_mapping();
                            let h_mapping = conn.halfedge_mapping();
                            match mesh.resolve_vertex_selection_full(&expr) {
                                Err(err) => err_label(ui, err),
                                Ok(verts) => {
                                    for v in verts {
                                        let vertex = &conn[v];
                                        ui.monospace(format!("--- Vertex {} ---", v_mapping[v]));
                                        ui.monospace(vertex.introspect(&h_mapping));
                                    }
                                }
                            }
                        }
                    },
                    ChannelKeyType::FaceId => match SelectionExpression::parse(&self.f_query) {
                        Err(err) => err_label(ui, err),
                        Ok(expr) => {
                            let conn = mesh.read_connectivity();
                            let f_mapping = conn.face_mapping();
                            let h_mapping = conn.halfedge_mapping();
                            match mesh.resolve_face_selection_full(&expr) {
                                Err(err) => err_label(ui, err),
                                Ok(faces) => {
                                    for f in faces {
                                        let face = &conn[f];
                                        ui.monospace(format!("--- Face {} ---", f_mapping[f]));
                                        ui.monospace(face.introspect(&h_mapping));
                                    }
                                }
                            }
                        }
                    },
                    ChannelKeyType::HalfEdgeId => match SelectionExpression::parse(&self.h_query) {
                        Err(err) => err_label(ui, err),
                        Ok(expr) => {
                            let conn = mesh.read_connectivity();
                            let h_mapping = conn.halfedge_mapping();
                            let v_mapping = conn.vertex_mapping();
                            let f_mapping = conn.face_mapping();
                            match mesh.resolve_halfedge_selection_full(&expr) {
                                Err(err) => err_label(ui, err),
                                Ok(halfedges) => {
                                    for h in halfedges {
                                        let halfedge = &conn[h];
                                        ui.monospace(format!("--- Halfedge {} ---", h_mapping[h]));
                                        ui.monospace(
                                            halfedge.introspect(&h_mapping, &v_mapping, &f_mapping),
                                        );
                                        ui.monospace("");
                                    }
                                }
                            }
                        }
                    },
                });
        }
    }
}
