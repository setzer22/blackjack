// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use egui::*;

/// This application renders several parts of the UI to offscreen textures which
/// are then drawin inside an egui widget. This is the widget that handles this
/// interaction.
///
/// Offscreen rendering happens with a 1-frame lag. First, egui lays out the
/// viewport widget and a size is computed and stored. Then, at the start of the
/// next frame, the offscreen image is rendered and a `TextureId` is generated.
/// This is then used to draw an `egui::Image` at the location.
pub struct AppViewport {
    pub rect: Rect,
    pub texture_id: Option<TextureId>,
}

impl AppViewport {
    pub fn new() -> AppViewport {
        AppViewport {
            // Don't create an empty rect, because this size will be used to
            // create a render target and may fail if resolution is zero.
            rect: Rect::from_min_size(Pos2::ZERO, vec2(10.0, 10.0)),
            texture_id: None,
        }
    }

    pub fn show(&mut self, ui: &mut Ui, desired_size: Vec2) {
        let (_, rect) = ui.allocate_space(desired_size);
        self.rect = rect;
        if let Some(texture_id) = self.texture_id {
            let mut mesh = epaint::Mesh::with_texture(texture_id);
            let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
            mesh.add_rect_with_uv(rect, uv, Color32::WHITE);
            ui.painter().add(Shape::mesh(mesh));
        } else {
            ui.painter().rect_filled(rect, 0.0, egui::Color32::RED);
        }
    }
}

impl Default for AppViewport {
    fn default() -> Self {
        Self::new()
    }
}
