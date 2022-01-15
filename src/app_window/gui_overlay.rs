use crate::prelude::*;
use egui::*;

// Need to divide by the pixels per point to accurately position on the
// screen at given coordinates.
pub fn project_point(
    render_ctx: &RenderContext,
    window_size: glam::Vec2,
    egui_ctx: &CtxRef,
    point: Vec3,
) -> Pos2 {
    let projected = render_ctx.project_point(point, window_size) / egui_ctx.pixels_per_point();
    egui::pos2(projected.x, projected.y)
}

pub fn draw_gui_overlays(
    render_ctx: &RenderContext,
    window_size: glam::Vec2,
    egui_ctx: &CtxRef,
    mesh: &HalfEdgeMesh,
) {
    let painter = egui_ctx.debug_painter();

    for (&v, mark) in mesh.iter_debug_vertices() {
        let point = mesh.vertex_position(v);
        let mut point = project_point(render_ctx, window_size, egui_ctx, point);
        point.y *= 0.5;
        
        painter.text(
            egui::pos2(point.x, point.y),
            egui::Align2::CENTER_BOTTOM,
            &mark.label,
            egui::TextStyle::Body,
            egui::Color32::WHITE,
        );
    }

    for (&h, mark) in mesh.iter_debug_halfedges() {
        let (src, dst) = mesh.at_halfedge(h).src_dst_pair().unwrap();
        let src_point = mesh.vertex_position(src);
        let dst_point = mesh.vertex_position(dst);
        let point = src_point * 0.333 + dst_point * 0.666;
        let mut point = project_point(render_ctx, window_size, egui_ctx, point);
        point.y *= 0.5;
        painter.text(
            egui::pos2(point.x, point.y),
            egui::Align2::CENTER_BOTTOM,
            &mark.label,
            egui::TextStyle::Body,
            egui::Color32::WHITE,
        );
    }
}
