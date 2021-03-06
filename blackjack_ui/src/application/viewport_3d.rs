// Copyright (C) 2022 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use blackjack_engine::prelude::HalfEdgeMesh;
use winit::event::MouseButton;

use crate::app_window::input::InputSystem;
use crate::{prelude::*, rendergraph};

use super::app_viewport::AppViewport;

#[derive(PartialEq, Eq)]
pub enum EdgeDrawMode {
    HalfEdge,
    FullEdge,
    None,
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
    None,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TextOverlayMode {
    /// No text overlay
    None,
    /// Display mesh information
    MeshInfo,
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
    view_proj: Mat4,
}

struct OrbitCamera {
    yaw: f32,
    pitch: f32,
    distance: f32,
    focus_point: Vec3,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            yaw: -30.0,
            pitch: 30.0,
            distance: 8.0,
            focus_point: Vec3::ZERO,
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
                overlay_mode: TextOverlayMode::None,
                render_vertices: true,
                matcap: 0,
            },
            view_proj: Mat4::default(),
        }
    }

    pub fn on_winit_event(
        &mut self,
        parent_scale: f32,
        viewport_rect: egui::Rect,
        event: winit::event::WindowEvent,
    ) {
        self.input
            .on_window_event(&event, parent_scale, viewport_rect);
    }

    fn update_camera(&mut self, render_ctx: &mut RenderContext) {
        // Update status
        if self.input.mouse.buttons().pressed(MouseButton::Left) {
            if self.input.shift_down {
                let cam_rotation = Mat4::from_rotation_y(self.camera.yaw.to_radians())
                    * Mat4::from_rotation_x(self.camera.pitch.to_radians());
                let camera_right = cam_rotation.transform_point3(Vec3::X);
                let camera_up = cam_rotation.transform_vector3(Vec3::Y);
                self.camera.focus_point += self.input.mouse.cursor_delta().x * camera_right * 0.1
                    + self.input.mouse.cursor_delta().y * -camera_up * 0.1;
            } else {
                self.camera.yaw += self.input.mouse.cursor_delta().x * 2.0;
                self.camera.pitch += self.input.mouse.cursor_delta().y * 2.0;
            }
        }
        self.camera.distance += self.input.mouse.wheel_delta() * 0.25;

        // Compute view matrix
        let view = Mat4::from_translation(Vec3::Z * self.camera.distance)
            * Mat4::from_rotation_x(-self.camera.pitch.to_radians())
            * Mat4::from_rotation_y(-self.camera.yaw.to_radians())
            * Mat4::from_translation(self.camera.focus_point);
        render_ctx.set_camera(view);
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
        self.view_proj = camera_manager.view_proj();

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

    fn get_resolution(&self) -> UVec2 {
        UVec2::new(
            (self.viewport_rect.width() * self.parent_scale) as u32,
            (self.viewport_rect.height() * self.parent_scale) as u32,
        )
    }

    pub fn add_to_graph<'node>(
        &'node mut self,
        graph: &mut r3::RenderGraph<'node>,
        ready: &r3::ReadyData,
        viewport_routines: super::ViewportRoutines<'node>,
    ) -> r3::RenderTargetHandle {
        rendergraph::blackjack_viewport_rendergraph(
            graph,
            ready,
            viewport_routines,
            self.get_resolution(),
            r3::SampleCount::One,
            Self::ambient_light(),
            &self.settings,
        )
    }

    pub fn show_ui(
        &mut self,
        ui: &mut egui::Ui,
        offscreen_viewport: &mut AppViewport,
        mesh: Option<&HalfEdgeMesh>,
    ) {
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
                            EdgeDrawMode::None,
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
                            FaceDrawMode::None,
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
                            TextOverlayMode::None,
                            "None",
                        );
                        ui.selectable_value(
                            &mut self.settings.overlay_mode,
                            TextOverlayMode::MeshInfo,
                            "Info",
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
        if let Some(mesh) = mesh {
            crate::app_window::gui_overlay::draw_gui_overlays(
                &self.view_proj,
                offscreen_viewport.rect,
                ui.ctx(),
                mesh,
                self.settings.overlay_mode,
            );
        }
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
