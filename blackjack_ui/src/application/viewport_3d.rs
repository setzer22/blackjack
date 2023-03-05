// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use blackjack_engine::lua_engine::RenderableThing;
use winit::event::MouseButton;

use crate::app_window::input::InputSystem;
use crate::{prelude::*, rendergraph};

use super::app_viewport::AppViewport;
use super::gizmo_ui::{self, GizmoViewportResponse, UiNodeGizmoStates};
use super::graph_editor::GraphEditor;

/// A generic lerper
mod lerp;
use lerp::*;

#[derive(PartialEq, Eq)]
pub enum EdgeDrawMode {
    HalfEdge,
    FullEdge,
    NoDraw,
}

#[derive(PartialEq, Eq)]
pub enum FaceDrawMode {
    /// Will read the actual configured value for the mesh and use its channel,
    /// if any. Defaults to flat shading otherwise.
    Real,
    /// Force flat shading, ignoring mesh data.
    Flat,
    /// Force smooth shading, ignoring mesh data
    Smooth,
    /// Don't draw faces.
    NoDraw,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TextOverlayMode {
    /// No text overlay
    NoDraw,
    /// Display face ids
    MeshInfoFaces,
    /// Display vertex ids
    MeshInfoVertices,
    /// Display halfedge ids
    MeshInfoHalfedges,
    /// Display all edge ids
    MeshInfoAll,
    /// Display mesh debug information set by the developers when debugging a
    /// problem. This is not intended to be used by regular users.
    DevDebug,
}

pub struct Viewport3dSettings {
    pub render_vertices: bool,
    pub matcap: usize,
    pub edge_mode: EdgeDrawMode,
    pub face_mode: FaceDrawMode,
    pub overlay_mode: TextOverlayMode,
}

pub struct Viewport3d {
    camera: OrbitCamera,
    input: InputSystem,
    viewport_rect: egui::Rect,
    parent_scale: f32,
    pub settings: Viewport3dSettings,
    view_proj_matrix: Mat4,
    view_matrix: Mat4,
    projection_matrix: Mat4,
    // True when a mouse drag does not belong to the camera. Such as when
    // dragging a gizmo.
    mouse_captured: bool,
}

struct OrbitCamera {
    yaw: Lerp<f32>,
    pitch: Lerp<f32>,
    distance: Lerp<f32>,
    fov: Lerp<f32>,
    focus_point: Lerp<Vec3>,
}

impl OrbitCamera {
    pub fn update(&mut self, delta: f32) {
        self.yaw.update(delta);
        self.pitch.update(delta);
        self.distance.update(delta);
        self.fov.update(delta * 2.0);
        self.focus_point.update(delta);
    }
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            yaw: Lerp::new(-30.0),
            pitch: Lerp::new(30.0),
            distance: Lerp::new(8.0),
            fov: Lerp::new(60.0),
            focus_point: Lerp::new(Vec3::ZERO),
        }
    }
}

impl Viewport3d {
    pub fn new() -> Self {
        Self {
            camera: OrbitCamera::default(),
            input: InputSystem::default(),
            // Initial size and scale is not important. It will get reset after
            // the first update.
            viewport_rect: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2::new(10.0, 10.0)),
            parent_scale: 1.0,
            settings: Viewport3dSettings {
                edge_mode: EdgeDrawMode::FullEdge,
                face_mode: FaceDrawMode::Real,
                overlay_mode: TextOverlayMode::NoDraw,
                render_vertices: true,
                matcap: 0,
            },
            view_proj_matrix: Mat4::default(),
            view_matrix: Mat4::default(),
            projection_matrix: Mat4::default(),
            mouse_captured: false,
        }
    }

    pub fn on_winit_event(
        &mut self,
        parent_scale: f32,
        viewport_rect: egui::Rect,
        event: winit::event::WindowEvent,
        mouse_captured_elsewhere: bool,
    ) {
        self.input.on_window_event(
            &event,
            parent_scale,
            viewport_rect,
            mouse_captured_elsewhere,
        );
    }

    fn update_camera(&mut self, render_ctx: &mut RenderContext) {
        const MIN_DIST: f32 = 0.1;
        const MAX_DIST: f32 = 120.0;

        self.camera.update(10.0 / 60.0);

        if !self.mouse_captured {
            // Update status
            if self.input.mouse.buttons().pressed(MouseButton::Left) {
                if self.input.shift_down {
                    let cam_rotation = Mat4::from_rotation_y(self.camera.yaw.get().to_radians())
                        * Mat4::from_rotation_x(self.camera.pitch.get().to_radians());
                    let camera_right = cam_rotation.transform_point3(Vec3::X);
                    let camera_up = cam_rotation.transform_vector3(Vec3::Y);
                    let move_speed = self.camera.distance.get() / MAX_DIST;
                    self.camera.focus_point +=
                        self.input.mouse.cursor_delta().x * camera_right * move_speed
                            + self.input.mouse.cursor_delta().y * -camera_up * move_speed;
                } else {
                    self.camera.yaw += self.input.mouse.cursor_delta().x * 2.0;
                    self.camera.pitch += self.input.mouse.cursor_delta().y * 2.0;
                }
            }
            self.camera.distance.set(|dist| {
                (dist - self.input.mouse.wheel_delta() * 0.5).clamp(MIN_DIST, MAX_DIST)
            });
            // self.camera
            // .fov
            // .set(|fov| (fov - self.input.mouse.wheel_delta() * 4.0).clamp(MIN_FOV, MAX_FOV));
        }

        // Compute view matrix
        let view = Mat4::from_translation(Vec3::Z * self.camera.distance.get())
            * Mat4::from_rotation_x(-self.camera.pitch.get().to_radians())
            * Mat4::from_rotation_y(-self.camera.yaw.get().to_radians())
            * Mat4::from_translation(self.camera.focus_point.get());
        render_ctx.set_camera(view, self.camera.fov.get());
    }

    pub fn update(
        &mut self,
        parent_scale: f32,
        viewport_rect: egui::Rect,
        render_ctx: &mut RenderContext,
    ) {
        self.viewport_rect = viewport_rect;
        self.parent_scale = parent_scale;

        self.update_camera(render_ctx);
        self.input.update();

        let camera_manager = &render_ctx.renderer.data_core.lock().camera_manager;
        self.view_proj_matrix = camera_manager.view_proj();
        self.view_matrix = camera_manager.view();
        self.projection_matrix = camera_manager.proj();

        // TODO: What if we ever have multiple 3d viewports? There's no way to
        // set the aspect ratio differently for different render passes in rend3
        // right now. The camera is global.
        //
        // See: https://github.com/BVE-Reborn/rend3/issues/327
        render_ctx
            .renderer
            .set_aspect_ratio(self.viewport_rect.width() / self.viewport_rect.height());
    }

    fn ambient_light() -> Vec4 {
        Vec4::splat(0.25)
    }

    pub fn get_resolution(&self) -> UVec2 {
        UVec2::new(
            (self.viewport_rect.width() * self.parent_scale) as u32,
            (self.viewport_rect.height() * self.parent_scale) as u32,
        )
    }

    /// Returns Some(render_target) when the viewport should be drawn by the
    /// calling context.
    pub fn add_to_graph<'node>(
        &'node mut self,
        graph: &mut r3::RenderGraph<'node>,
        ready: &r3::ReadyData,
        viewport_routines: super::ViewportRoutines<'node>,
    ) -> Option<r3::RenderTargetHandle> {
        let resolution = self.get_resolution();
        if resolution.x == 0 || resolution.y == 0 {
            None
        } else {
            Some(rendergraph::blackjack_viewport_rendergraph(
                graph,
                ready,
                viewport_routines,
                self.get_resolution(),
                r3::SampleCount::One,
                Self::ambient_light(),
                &self.settings,
            ))
        }
    }

    pub fn show_ui(
        &mut self,
        ui: &mut egui::Ui,
        offscreen_viewport: &mut AppViewport,
        renderable_thing: Option<&RenderableThing>,
        graph_editor: &GraphEditor,
        node_gizmo_states: &mut UiNodeGizmoStates,
    ) -> Result<()> {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                mesh_visuals_popup(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Edges:");
                        ui.selectable_value(
                            &mut self.settings.edge_mode,
                            EdgeDrawMode::FullEdge,
                            "Full",
                        );
                        ui.selectable_value(
                            &mut self.settings.edge_mode,
                            EdgeDrawMode::HalfEdge,
                            "Half",
                        );
                        ui.selectable_value(
                            &mut self.settings.edge_mode,
                            EdgeDrawMode::NoDraw,
                            "None",
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("Vertices:");
                        ui.checkbox(&mut self.settings.render_vertices, "");
                    });

                    ui.horizontal(|ui| {
                        ui.label("Faces:");
                        ui.selectable_value(
                            &mut self.settings.face_mode,
                            FaceDrawMode::Real,
                            "Real",
                        );
                        ui.selectable_value(
                            &mut self.settings.face_mode,
                            FaceDrawMode::Flat,
                            "Flat",
                        );
                        ui.selectable_value(
                            &mut self.settings.face_mode,
                            FaceDrawMode::Smooth,
                            "Smooth",
                        );
                        ui.selectable_value(
                            &mut self.settings.face_mode,
                            FaceDrawMode::NoDraw,
                            "None",
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("Matcap:");
                        if ui.button("<").clicked() {
                            self.settings.matcap -= 1;
                        }
                        ui.add(
                            egui::DragValue::new(&mut self.settings.matcap)
                                .clamp_range(0..=crate::rendergraph::face_routine::NUM_MATCAPS - 1),
                        );
                        if ui.button(">").clicked() {
                            self.settings.matcap += 1;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Text Overlay:");
                        ui.selectable_value(
                            &mut self.settings.overlay_mode,
                            TextOverlayMode::NoDraw,
                            "None",
                        );
                        ui.selectable_value(
                            &mut self.settings.overlay_mode,
                            TextOverlayMode::MeshInfoVertices,
                            "V",
                        );
                        ui.selectable_value(
                            &mut self.settings.overlay_mode,
                            TextOverlayMode::MeshInfoFaces,
                            "F",
                        );
                        ui.selectable_value(
                            &mut self.settings.overlay_mode,
                            TextOverlayMode::MeshInfoHalfedges,
                            "H",
                        );
                        ui.selectable_value(
                            &mut self.settings.overlay_mode,
                            TextOverlayMode::MeshInfoAll,
                            "A",
                        );
                        ui.selectable_value(
                            &mut self.settings.overlay_mode,
                            TextOverlayMode::DevDebug,
                            "Debug",
                        );
                    });
                });
            });
            offscreen_viewport.show(ui, ui.available_size());
        });
        if let Some(renderable_thing) = renderable_thing {
            crate::app_window::gui_overlay::draw_gui_overlays(
                &self.view_proj_matrix,
                offscreen_viewport.rect,
                ui.ctx(),
                renderable_thing,
                self.settings.overlay_mode,
            );

            self.mouse_captured = false;
            node_gizmo_states.iterate_gizmos_for_drawing(
                |node_id, gizmo_idx, gizmo, has_focus| {
                    let node = &graph_editor.editor_state.graph[node_id];
                    let responses = gizmo_ui::draw_gizmo_ui_viewport(
                        self,
                        ui,
                        gizmo,
                        (node_id, gizmo_idx),
                        node,
                        has_focus,
                    )?;
                    let mut gizmos_changed = false;

                    for response in responses {
                        match response {
                            GizmoViewportResponse::CaptureMouse => {
                                self.mouse_captured = true;
                            }
                            GizmoViewportResponse::GizmoIsInteracted => {
                                gizmos_changed = true;
                            }
                        }
                    }
                    Ok(gizmos_changed)
                },
            )?;
        }
        Ok(())
    }

    pub fn view_matrix(&self) -> Mat4 {
        self.view_matrix
    }

    pub fn projection_matrix(&self) -> Mat4 {
        self.projection_matrix
    }

    pub fn viewport_rect(&self) -> egui::Rect {
        self.viewport_rect
    }
}

/// Draws the "Mesh Visuals" popup.
/// This code was adapted from egui's Color Picker widget
pub fn mesh_visuals_popup(
    ui: &mut egui::Ui,
    contents: impl FnOnce(&mut egui::Ui),
) -> egui::Response {
    let popup_id = egui::Id::new("settings_popup");
    let mut button_response = ui.button("Mesh Visuals");
    if ui.style().explanation_tooltips {
        button_response = button_response.on_hover_text("Click to edit mesh visuals");
    }

    if button_response.clicked() {
        ui.memory().toggle_popup(popup_id);
    }
    if ui.memory().is_popup_open(popup_id) {
        let area_response = egui::Area::new(popup_id)
            .order(egui::Order::Foreground)
            .default_pos(button_response.rect.left_bottom() + egui::vec2(0.0, 10.0))
            .show(ui.ctx(), |ui| {
                ui.spacing_mut().slider_width = 210.0;
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    contents(ui);
                });
            })
            .response;

        if !button_response.clicked()
            && (ui.input().key_pressed(egui::Key::Escape) || area_response.clicked_elsewhere())
        {
            ui.memory().close_popup();
        }
    }

    button_response
}

impl Default for Viewport3d {
    fn default() -> Self {
        Self::new()
    }
}
