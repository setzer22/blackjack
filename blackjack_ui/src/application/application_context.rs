// Copyright (C) 2022 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::graph::graph_interop;
use crate::prelude::*;
use anyhow::Error;
use blackjack_engine::{
    lua_engine::{LuaRuntime, ProgramResult, RenderableThing},
    prelude::{FaceOverlayBuffers, LineBuffers, PointBuffers, VertexIndexBuffers},
};
use egui_node_graph::NodeId;

use super::{
    gizmo_ui::BlackjackUiGizmo,
    root_ui::AppRootAction,
    viewport_3d::{EdgeDrawMode, FaceDrawMode, Viewport3dSettings},
    viewport_split::SplitTree,
};

pub struct ApplicationContext {
    /// The 'renderable thing' is at the center of the application, it is
    /// typically a kind of mesh.
    /// - The graph generates a program that produces it.
    /// - The 3d viewport renders it.
    pub renderable_thing: Option<RenderableThing>,
    /// The currently active gizmos. Gizmos are returned by nodes to represent
    /// visual objects that can be used to manipulate its parameters.
    pub active_gizmos: Vec<Box<dyn BlackjackUiGizmo>>,
    /// The tree of splits at the center of application. Splits recursively
    /// partition the state either horizontally or vertically. This separation
    /// is dynamic, very similar to Blender's UI model
    pub split_tree: SplitTree,
}

impl ApplicationContext {
    pub fn new() -> ApplicationContext {
        ApplicationContext {
            renderable_thing: None,
            active_gizmos: Vec::new(),
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
        model_matrix: Mat4,
    ) -> Vec<AppRootAction> {
        // TODO: Instead of clearing all objects, make the app context own the
        // objects it's drawing and clear those instead.
        render_ctx.clear_objects();

        let mut actions = vec![];

        match self.run_active_node(editor_state, custom_state, lua_runtime) {
            Ok(code) => {
                actions.push(AppRootAction::SetCodeViewerCode(code));
            }
            Err(err) => {
                self.paint_errors(egui_ctx, err);
            }
        };

        if let Some(RenderableThing::HalfEdgeMesh(m)) = &self.renderable_thing {
            let (s, r, t) = model_matrix.to_scale_rotation_translation();
            blackjack_engine::mesh::halfedge::edit_ops::transform(
                m,
                t,
                Vec3::from(r.to_euler(glam::EulerRot::XYZ)),
                s,
            )
            .expect("This code should not make it to review...");
        }

        if let Err(err) = self.run_side_effects(editor_state, custom_state, lua_runtime) {
            eprintln!(
                "There was an errror executing side effect: {err}\nBacktrace:\n----------\n{}",
                err.backtrace()
            );
        }
        if let Err(err) = self.build_and_render_mesh(render_ctx, viewport_settings) {
            self.paint_errors(egui_ctx, err);
        }

        actions
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
                        FaceDrawMode::None => None,
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

                // Face overlays
                {
                    let FaceOverlayBuffers { positions, colors } =
                        mesh.generate_face_overlay_buffers();
                    if !positions.is_empty() {
                        render_ctx.face_routine.add_overlay_mesh(
                            &render_ctx.renderer,
                            &positions,
                            &colors,
                        );
                    }
                }

                // Edges
                {
                    if let Some(LineBuffers { positions, colors }) =
                        match viewport_settings.edge_mode {
                            EdgeDrawMode::HalfEdge => Some(mesh.generate_halfedge_arrow_buffers()?),
                            EdgeDrawMode::FullEdge => Some(mesh.generate_line_buffers()?),
                            EdgeDrawMode::None => None,
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
        painter.text(
            egui::pos2(width - 10.0, 30.0),
            egui::Align2::RIGHT_TOP,
            format!("{}", err),
            egui::FontId::default(),
            egui::Color32::RED,
        );
    }

    pub fn run_node(
        &self,
        graph: &graph::Graph,
        lua_runtime: &LuaRuntime,
        node: NodeId,
    ) -> Result<ProgramResult> {
        let (bjk_graph, mapping) =
            graph_interop::ui_graph_to_blackjack_graph(graph, &lua_runtime.node_definitions)?;
        let params = graph_interop::extract_graph_params(graph, &bjk_graph, &mapping)?;
        blackjack_engine::graph_interpreter::run_graph(
            &lua_runtime.lua,
            &bjk_graph,
            mapping[node],
            &params,
        )
    }

    // Returns the compiled lua code
    pub fn run_active_node(
        &mut self,
        editor_state: &graph::GraphEditorState,
        custom_state: &mut graph::CustomGraphState,
        lua_runtime: &LuaRuntime,
    ) -> Result<String> {
        if let Some(active) = custom_state.active_node {
            let program_result = self.run_node(&editor_state.graph, lua_runtime, active)?;
            self.renderable_thing = program_result.renderable;
            // TODO REVIEW: Remove the whole "code" tab
            Ok("".into())
        } else {
            self.renderable_thing = None;
            Ok("".into())
        }
    }

    pub fn run_side_effects(
        &mut self,
        editor_state: &mut graph::GraphEditorState,
        custom_state: &mut graph::CustomGraphState,
        lua_runtime: &LuaRuntime,
    ) -> Result<()> {
        if let Some(side_effect) = custom_state.run_side_effect.take() {
            // We ignore the result. The program is only executed to produce a
            // side effect (e.g. exporting a mesh as OBJ)
            let _ = self.run_node(&editor_state.graph, lua_runtime, side_effect)?;
        }
        Ok(())
    }
}

impl Default for ApplicationContext {
    fn default() -> Self {
        Self::new()
    }
}
