// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::graph::graph_interop::{self, NodeMapping};
use crate::prelude::*;
use anyhow::Error;

use blackjack_engine::graph::BjkGraph;
use blackjack_engine::graph_interpreter::ExternalParameterValues;
use blackjack_engine::prelude::ChannelKeyType;
use blackjack_engine::{
    lua_engine::{LuaRuntime, RenderableThing},
    prelude::{FaceOverlayBuffers, LineBuffers, PointBuffers, VertexIndexBuffers},
};
use egui::epaint::RectShape;
use egui::{Rounding, Shape};

use super::gizmo_ui::UiNodeGizmoStates;
use super::{
    root_ui::AppRootAction,
    viewport_3d::{EdgeDrawMode, FaceDrawMode, Viewport3dSettings},
    viewport_split::SplitTree,
};

pub struct MeshViewportSelection {
    pub hovered: Option<u32>,
    pub selected: HashSet<u32>,
    pub primitive_type: ChannelKeyType,
}

pub struct ApplicationContext {
    /// The 'renderable thing' is at the center of the application, it is
    /// typically a kind of mesh.
    /// - The graph generates a program that produces it.
    /// - The 3d viewport renders it.
    pub renderable_thing: Option<RenderableThing>,
    /// If the current `renderable_thing` is a HalfEdgeMesh and there is
    /// currently a request to select a group of primitives in the viewport,
    /// this stores the data for the selection.
    pub current_selection: Option<MeshViewportSelection>,
    /// The currently active gizmos. Gizmos are returned by nodes to represent
    /// visual objects that can be used to manipulate its parameters.
    pub node_gizmo_states: UiNodeGizmoStates,
    /// The tree of splits at the center of application. Splits recursively
    /// partition the state either horizontally or vertically. This separation
    /// is dynamic, very similar to Blender's UI model
    pub split_tree: SplitTree,
}

impl ApplicationContext {
    pub fn new(gizmo_states: UiNodeGizmoStates) -> ApplicationContext {
        ApplicationContext {
            renderable_thing: None,
            current_selection: None,
            node_gizmo_states: gizmo_states,
            split_tree: SplitTree::default_tree(),
        }
    }

    pub fn setup(&self, render_ctx: &mut RenderContext) {
        render_ctx.add_light(r3::DirectionalLight {
            color: glam::Vec3::ONE,
            intensity: 10.0,
            // Direction will be normalized
            direction: glam::Vec3::new(-1.0, -4.0, 2.0),
            distance: 400.0,
        });
    }

    pub fn update(
        &mut self,
        egui_ctx: &egui::Context,
        editor_state: &mut graph::GraphEditorState,
        custom_state: &mut graph::CustomGraphState,
        render_ctx: &mut RenderContext,
        viewport_settings: &Viewport3dSettings,
        lua_runtime: &LuaRuntime,
    ) -> Vec<AppRootAction> {
        // TODO: Instead of clearing all objects, make the app context own the
        // objects it's drawing and clear those instead.
        render_ctx.clear_objects();

        if let Err(err) = self.run_active_node(editor_state, custom_state, lua_runtime) {
            self.paint_errors(egui_ctx, err);
        };

        if let Err(err) = self.run_side_effects(editor_state, custom_state, lua_runtime) {
            eprintln!(
                "There was an errror executing side effect: {err}\nBacktrace:\n----------\n{}",
                err.backtrace()
            );
        }
        if let Err(err) = self.build_and_render_mesh(render_ctx, viewport_settings) {
            self.paint_errors(egui_ctx, err);
        }

        Vec::new()
    }

    pub fn build_and_render_mesh(
        &mut self,
        render_ctx: &mut RenderContext,
        viewport_settings: &Viewport3dSettings,
    ) -> Result<()> {
        match self.renderable_thing.as_mut() {
            Some(RenderableThing::HalfEdgeMesh(mesh)) => {
                // Base mesh
                {
                    if let Some(VertexIndexBuffers {
                        positions,
                        normals,
                        indices,
                    }) = match viewport_settings.face_mode {
                        FaceDrawMode::Real => {
                            if mesh.gen_config.smooth_normals {
                                Some(mesh.generate_triangle_buffers_smooth(false)?)
                            } else {
                                Some(mesh.generate_triangle_buffers_flat(false)?)
                            }
                        }
                        FaceDrawMode::Flat => Some(mesh.generate_triangle_buffers_flat(true)?),
                        FaceDrawMode::Smooth => Some(mesh.generate_triangle_buffers_smooth(true)?),
                        FaceDrawMode::NoDraw => None,
                    } {
                        if !positions.is_empty() {
                            render_ctx.face_routine.add_base_mesh(
                                &render_ctx.renderer,
                                &positions,
                                &normals,
                                &indices,
                            );
                        }
                    }
                }

                // Face overlays and ids
                {
                    let FaceOverlayBuffers {
                        positions,
                        colors,
                        ids,
                        max_id,
                    } = mesh.generate_face_overlay_buffers(
                        self.current_selection.as_ref().and_then(|x| x.hovered),
                    );
                    if !positions.is_empty() {
                        render_ctx.face_routine.add_overlay_mesh(
                            &render_ctx.renderer,
                            &positions,
                            &colors,
                            &ids,
                            max_id,
                        );
                    }
                }

                // Edges
                {
                    if let Some(LineBuffers { positions, colors }) =
                        match viewport_settings.edge_mode {
                            EdgeDrawMode::HalfEdge => Some(mesh.generate_halfedge_arrow_buffers()?),
                            EdgeDrawMode::FullEdge => Some(mesh.generate_line_buffers()?),
                            EdgeDrawMode::NoDraw => None,
                        }
                    {
                        if !positions.is_empty() {
                            render_ctx.wireframe_routine.add_wireframe(
                                &render_ctx.renderer.device,
                                &positions,
                                &colors,
                            )
                        }
                    }
                }

                // Vertices
                {
                    let PointBuffers { positions } = mesh.generate_point_buffers();
                    if !positions.is_empty() {
                        render_ctx
                            .point_cloud_routine
                            .add_point_cloud(&render_ctx.renderer.device, &positions);
                    }
                }
            }
            Some(RenderableThing::HeightMap(heightmap)) => {
                let VertexIndexBuffers {
                    positions,
                    normals,
                    indices,
                } = heightmap.generate_triangle_buffers();

                if !positions.is_empty() {
                    render_ctx.face_routine.add_base_mesh(
                        &render_ctx.renderer,
                        &positions,
                        &normals,
                        &indices,
                    );
                }
            }
            None => { /* Ignore */ }
        }
        Ok(())
    }

    pub fn paint_errors(&mut self, egui_ctx: &egui::Context, err: Error) {
        let painter = egui_ctx.debug_painter();
        let width = egui_ctx.available_rect().width();
        let bg_shape = painter.add(Shape::Noop);
        let text_rect = painter.text(
            egui::pos2(width - 10.0, 30.0),
            egui::Align2::RIGHT_TOP,
            format!("{err}"),
            egui::FontId::default(),
            egui::Color32::RED,
        );
        painter.set(
            bg_shape,
            Shape::Rect(RectShape {
                rect: text_rect.expand(5.0),
                rounding: Rounding::none(),
                fill: egui::Color32::from_rgba_unmultiplied(40, 40, 40, 240),
                stroke: egui::Stroke::none(),
            }),
        )
    }

    pub fn generate_bjk_graph(
        &self,
        graph: &graph::Graph,
        custom_state: &graph::CustomGraphState,
    ) -> Result<(BjkGraph, NodeMapping, ExternalParameterValues)> {
        let (bjk_graph, mapping) = graph_interop::ui_graph_to_blackjack_graph(graph, custom_state)?;
        let params = graph_interop::extract_graph_params(graph, &bjk_graph, &mapping)?;
        Ok((bjk_graph, mapping, params))
    }

    // Returns the compiled lua code
    pub fn run_active_node(
        &mut self,
        editor_state: &mut graph::GraphEditorState,
        custom_state: &mut graph::CustomGraphState,
        lua_runtime: &LuaRuntime,
    ) -> Result<()> {
        if let Some(active) = custom_state.active_node {
            let (bjk_graph, mapping, params) =
                self.generate_bjk_graph(&editor_state.graph, custom_state)?;
            let gizmos = self.node_gizmo_states.to_bjk_data(&mapping);
            let program_result = blackjack_engine::graph_interpreter::run_graph(
                &lua_runtime.lua,
                &bjk_graph,
                mapping[active],
                params,
                &lua_runtime.node_definitions,
                Some(gizmos),
            )?;

            self.renderable_thing = program_result.renderable;
            if let Some(updated_gizmos) = program_result.updated_gizmos {
                self.node_gizmo_states
                    .update_gizmos(updated_gizmos, &mapping)?;
            }

            // TODO: This is debug code used by viewport picking. Currently disabled.
            /* if let Some(RenderableThing::HalfEdgeMesh(_)) = &self.renderable_thing {
                if self.current_selection.is_none() {
                    self.current_selection = Some(MeshViewportSelection {
                        hovered: None,
                        selected: HashSet::new(),
                        primitive_type: ChannelKeyType::FaceId,
                    });
                }
            } */

            // Running gizmos returns a set of updated values, we need to
            // refresh the UI graph values with those here.
            graph_interop::set_parameters_from_external_values(
                &mut editor_state.graph,
                program_result.updated_values,
                mapping,
            )?;
        } else {
            self.renderable_thing = None;
        }
        Ok(())
    }

    pub fn run_side_effects(
        &mut self,
        editor_state: &mut graph::GraphEditorState,
        custom_state: &mut graph::CustomGraphState,
        lua_runtime: &LuaRuntime,
    ) -> Result<()> {
        if let Some(side_effect) = custom_state.run_side_effect.take() {
            let (bjk_graph, mapping, params) =
                self.generate_bjk_graph(&editor_state.graph, custom_state)?;
            // We ignore the result. The program is only executed to produce a
            // side effect (e.g. exporting a mesh as OBJ)
            let _ = blackjack_engine::graph_interpreter::run_graph(
                &lua_runtime.lua,
                &bjk_graph,
                mapping[side_effect],
                params,
                &lua_runtime.node_definitions,
                None,
            )?;
        }
        Ok(())
    }

    pub fn on_id_hovered(&mut self, id: Option<u32>) {
        if let Some(selection) = &mut self.current_selection {
            selection.hovered = id;
        }
    }
}
