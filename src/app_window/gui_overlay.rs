use crate::{prelude::*, application::viewport_3d::TextOverlayMode};
use egui::*;

// Need to divide by the pixels per point to accurately position on the
// screen at given coordinates.
pub fn project_point(
    view_proj: &Mat4,
    viewport_rect: Rect,
    egui_ctx: &CtxRef,
    point: Vec3,
) -> Pos2 {
    let size = glam::Vec2::new(viewport_rect.size().x, viewport_rect.size().y);
    let offset = glam::Vec2::new(viewport_rect.left_top().x, viewport_rect.left_top().y);
    let projected =
        RenderContext::project_point(view_proj, point, size, offset) / egui_ctx.pixels_per_point();
    egui::pos2(projected.x, projected.y)
}

pub fn draw_gui_overlays(
    view_proj: &Mat4,
    viewport_rect: egui::Rect,
    egui_ctx: &CtxRef,
    mesh: &HalfEdgeMesh,
    overlay_type: TextOverlayMode,
) {
    let painter = egui_ctx.debug_painter();

    let conn = mesh.read_connectivity();
    let positions = mesh.read_positions();

    let text = |point: Pos2, text: &str| {
        painter.text(
            point,
            egui::Align2::CENTER_BOTTOM,
            text,
            egui::TextStyle::Body,
            egui::Color32::WHITE,
        );
    };

    match overlay_type {
        TextOverlayMode::None => {}
        TextOverlayMode::MeshInfo => {
            for (i, (v, _)) in conn.iter_vertices().enumerate() {
                text(
                    project_point(view_proj, viewport_rect, egui_ctx, positions[v]),
                    &format!("v{i}"),
                )
            }
            for (i, (h, _)) in conn.iter_halfedges().enumerate() {
                let (src, dst) = conn.at_halfedge(h).src_dst_pair().unwrap();
                let src_point = positions[src];
                let dst_point = positions[dst];
                let point = src_point * 0.333 + dst_point * 0.666;
                text(
                    project_point(view_proj, viewport_rect, egui_ctx, point),
                    &format!("h{i}"),
                )
            }
            for (i, (f, _)) in conn.iter_faces().enumerate() {
                let point = conn.face_vertex_average(&positions, f);
                text(
                    project_point(view_proj, viewport_rect, egui_ctx, point),
                    &format!("f{i}"),
                )
            }
        }
        TextOverlayMode::DevDebug => {
            for (&v, mark) in conn.iter_debug_vertices() {
                text(
                    project_point(view_proj, viewport_rect, egui_ctx, positions[v]),
                    &mark.label,
                );
            }

            for (&h, mark) in conn.iter_debug_halfedges() {
                let (src, dst) = conn.at_halfedge(h).src_dst_pair().unwrap();
                let src_point = positions[src];
                let dst_point = positions[dst];
                let point = src_point * 0.333 + dst_point * 0.666;
                text(
                    project_point(view_proj, viewport_rect, egui_ctx, point),
                    &mark.label,
                );
            }
        }
    }
}
