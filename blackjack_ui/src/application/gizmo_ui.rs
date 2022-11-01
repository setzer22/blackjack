use anyhow::Result;
use blackjack_engine::gizmos::BlackjackGizmo;
use glam::Mat4;

use super::viewport_3d::Viewport3d;

pub enum GizmoViewportResponse {
    MouseDragged,
}

pub fn draw_gizmo_ui_viewport(
    viewport: &Viewport3d,
    ui: &mut egui::Ui,
    gizmo: &mut BlackjackGizmo,
) -> Result<Vec<GizmoViewportResponse>> {
    let mut responses = Vec::new();

    match gizmo {
        BlackjackGizmo::Transform(transform_gizmo) => {
            dbg!(&transform_gizmo);
            let gizmo = egui_gizmo::Gizmo::new("viewport_gizmo")
                .view_matrix(viewport.view_matrix().to_cols_array_2d())
                .projection_matrix(viewport.projection_matrix().to_cols_array_2d())
                .model_matrix(transform_gizmo.matrix().to_cols_array_2d())
                .viewport(viewport.viewport_rect())
                .mode(egui_gizmo::GizmoMode::Rotate);
            if let Some(response) = gizmo.interact(ui) {
                responses.push(GizmoViewportResponse::MouseDragged);
                let updated_matrix = Mat4::from_cols_array_2d(&response.transform);
                transform_gizmo.set_from_matrix(updated_matrix);
            }
        }
    }

    Ok(responses)
}
