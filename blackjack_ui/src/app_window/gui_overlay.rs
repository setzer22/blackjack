// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{application::viewport_3d::TextOverlayMode, prelude::*};
use blackjack_engine::{lua_engine::RenderableThing, prelude::HalfedgeTraversalHelpers};
use egui::*;

// Need to divide by the pixels per point to accurately position on the
// screen at given coordinates.
pub fn project_point(view_proj: &Mat4, viewport_rect: Rect, point: Vec3) -> Pos2 {
    let size = glam::Vec2::new(viewport_rect.size().x, viewport_rect.size().y);
    let offset = glam::Vec2::new(viewport_rect.left_top().x, viewport_rect.left_top().y);
    let projected = RenderContext::project_point(view_proj, point, size, offset);
    egui::pos2(projected.x, projected.y)
}

pub fn draw_gui_overlays(
    view_proj: &Mat4,
    viewport_rect: egui::Rect,
    egui_ctx: &egui::Context,
    renderable_thing: &RenderableThing,
    overlay_type: TextOverlayMode,
) {
    match renderable_thing {
        RenderableThing::HalfEdgeMesh(mesh) => {
            let painter = egui_ctx.debug_painter();

            let conn = mesh.read_connectivity();
            let positions = mesh.read_positions();

            let text = |point: Pos2, text: &str| {
                painter.text(
                    point,
                    egui::Align2::CENTER_BOTTOM,
                    text,
                    egui::FontId::default(),
                    egui::Color32::WHITE,
                );
            };

            match overlay_type {
                TextOverlayMode::NoDraw => {}
                TextOverlayMode::MeshInfoVertices
                | TextOverlayMode::MeshInfoFaces
                | TextOverlayMode::MeshInfoHalfedges
                | TextOverlayMode::MeshInfoAll => {
                    if matches!(
                        overlay_type,
                        TextOverlayMode::MeshInfoAll | TextOverlayMode::MeshInfoVertices
                    ) {
                        for (i, (v, _)) in conn.iter_vertices().enumerate() {
                            text(
                                project_point(view_proj, viewport_rect, positions[v]),
                                &format!("v{i}"),
                            )
                        }
                    }
                    if matches!(
                        overlay_type,
                        TextOverlayMode::MeshInfoAll | TextOverlayMode::MeshInfoHalfedges
                    ) {
                        for (i, (h, _)) in conn.iter_halfedges().enumerate() {
                            let (src, dst) = conn.at_halfedge(h).src_dst_pair().unwrap();
                            let src_point = positions[src];
                            let dst_point = positions[dst];
                            let point = src_point * 0.333 + dst_point * 0.666;
                            text(
                                project_point(view_proj, viewport_rect, point),
                                &format!("h{i}"),
                            )
                        }
                    }

                    if matches!(
                        overlay_type,
                        TextOverlayMode::MeshInfoAll | TextOverlayMode::MeshInfoFaces
                    ) {
                        for (i, (f, _)) in conn.iter_faces().enumerate() {
                            let point = conn.face_vertex_average(&positions, f);
                            text(
                                project_point(view_proj, viewport_rect, point),
                                &format!("f{i}"),
                            )
                        }
                    }
                }
                TextOverlayMode::DevDebug => {
                    for (&v, mark) in conn.iter_debug_vertices() {
                        text(
                            project_point(view_proj, viewport_rect, positions[v]),
                            &mark.label,
                        );
                    }

                    for (&h, mark) in conn.iter_debug_halfedges() {
                        let (src, dst) = conn.at_halfedge(h).src_dst_pair().unwrap();
                        let src_point = positions[src];
                        let dst_point = positions[dst];
                        let point = src_point * 0.333 + dst_point * 0.666;
                        text(project_point(view_proj, viewport_rect, point), &mark.label);
                    }
                }
            }
        }
        RenderableThing::HeightMap(_) => {
            // TODO @Heightmap
        }
    }
}
