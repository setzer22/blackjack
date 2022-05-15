use crate::prelude::*;
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
) {
    let painter = egui_ctx.debug_painter();

    let conn = mesh.read_connectivity();
    let positions = mesh.read_positions();

    for (&v, mark) in conn.iter_debug_vertices() {
        let point = positions[v];
        let point = project_point(view_proj, viewport_rect, egui_ctx, point);

        painter.text(
            egui::pos2(point.x, point.y),
            egui::Align2::CENTER_BOTTOM,
            &mark.label,
            egui::TextStyle::Body,
            egui::Color32::WHITE,
        );
    }

    for (&h, mark) in conn.iter_debug_halfedges() {
        let (src, dst) = conn.at_halfedge(h).src_dst_pair().unwrap();
        let src_point = positions[src];
        let dst_point = positions[dst];
        let point = src_point * 0.333 + dst_point * 0.666;
        let point = project_point(view_proj, viewport_rect, egui_ctx, point);
        painter.text(
            egui::pos2(point.x, point.y),
            egui::Align2::CENTER_BOTTOM,
            &mark.label,
            egui::TextStyle::Body,
            egui::Color32::WHITE,
        );
    }
}
