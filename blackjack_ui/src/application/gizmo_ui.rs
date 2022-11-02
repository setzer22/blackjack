use anyhow::Result;
use blackjack_engine::gizmos::{BlackjackGizmo, TransformGizmoMode};
use glam::Mat4;

use super::viewport_3d::Viewport3d;

pub enum GizmoViewportResponse {
    CaptureMouse,
    GizmoIsInteracted,
}

pub fn draw_gizmo_ui_viewport(
    viewport: &Viewport3d,
    ui: &mut egui::Ui,
    gizmo: &mut BlackjackGizmo,
) -> Result<Vec<GizmoViewportResponse>> {
    let mut responses = Vec::new();

    match gizmo {
        BlackjackGizmo::Transform(transform_gizmo) => {
            ui.allocate_ui_at_rect(viewport.viewport_rect().shrink(10.0), |ui| {
                if ui.button("Move (G)").clicked() || ui.input().key_pressed(egui::Key::G) {
                    transform_gizmo.gizmo_mode = TransformGizmoMode::Translate;
                }
                if ui.button("Rotate (R)").clicked() || ui.input().key_pressed(egui::Key::R) {
                    transform_gizmo.gizmo_mode = TransformGizmoMode::Rotate;
                }
                if ui.button("Scale (S)").clicked() || ui.input().key_pressed(egui::Key::S) {
                    transform_gizmo.gizmo_mode = TransformGizmoMode::Scale;
                }
            });

            let gizmo = egui_gizmo::Gizmo::new("viewport_gizmo")
                .view_matrix(viewport.view_matrix().to_cols_array_2d())
                .projection_matrix(viewport.projection_matrix().to_cols_array_2d())
                .model_matrix(transform_gizmo.matrix().to_cols_array_2d())
                .viewport(viewport.viewport_rect())
                .mode(match transform_gizmo.gizmo_mode {
                    TransformGizmoMode::Translate => egui_gizmo::GizmoMode::Translate,
                    TransformGizmoMode::Rotate => egui_gizmo::GizmoMode::Rotate,
                    TransformGizmoMode::Scale => egui_gizmo::GizmoMode::Scale,
                });
            if let Some(response) = gizmo.interact(ui) {
                responses.push(GizmoViewportResponse::CaptureMouse);
                responses.push(GizmoViewportResponse::GizmoIsInteracted);
                let updated_matrix = Mat4::from_cols_array_2d(&response.transform);
                transform_gizmo.set_from_matrix(updated_matrix);
            }
        }
    }

    Ok(responses)
}
