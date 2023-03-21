// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::*;
use std::path::PathBuf;

pub enum AppRootAction {
    Save(PathBuf),
    Load(PathBuf),
}

impl RootViewport {
    pub fn top_menubar(&mut self) -> Option<AppRootAction> {
        let mut action = None;
        egui::TopBottomPanel::top("top_menubar").show(&self.egui_context, |ui| {
            // When set, will load a new editor state at the end of this function
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    ui.add_enabled_ui(false, |ui| ui.button("New"));
                    if ui.button("Open…").clicked() {
                        let file_location = rfd::FileDialog::new()
                            .add_filter("Blackjack Model", &["bjk"])
                            .pick_file();
                        if let Some(path) = file_location {
                            action = Some(AppRootAction::Load(path))
                        }
                    }
                    ui.separator();
                    if ui.button("Save As…").clicked() {
                        let file_location = rfd::FileDialog::new()
                            .set_file_name("Untitled.bjk")
                            .add_filter("Blackjack Model", &["bjk"])
                            .save_file();
                        if let Some(path) = file_location {
                            action = Some(AppRootAction::Save(path))
                        }
                    }
                    ui.separator();
                    ui.add_enabled_ui(false, |ui| ui.button("Quit"));
                });
                ui.menu_button("Window", |ui| {
                    ui.checkbox(&mut self.diagnostics_open, "Diagnostics");
                });
            });
        });

        action
    }

    pub fn diagnostics_ui(&mut self) {
        egui::Window::new("Diagnostics")
            .open(&mut self.diagnostics_open)
            .show(&self.egui_context, |ui| {
                ui.label(format!("HiDPI Scale: {}", ui.ctx().pixels_per_point()));
            });
    }

    pub fn show_leaf(ui: &mut egui::Ui, payload: &mut Self, name: &str) {
        // TODO: These names here are hard-coded in the creation of the
        // SplitTree. We should be using some kind of identifier instead
        match name {
            "3d_view" => {
                if let Err(err) = payload.viewport_3d.show_ui(
                    ui,
                    payload
                        .offscreen_viewports
                        .get_mut(&OffscreenViewport::Viewport3d)
                        .unwrap(),
                    payload.app_context.renderable_thing.as_ref(),
                    &payload.graph_editor,
                    &mut payload.app_context.node_gizmo_states,
                ) {
                    // TODO: Do something better for error reporting
                    println!("Error in viewport: {err}")
                }
            }
            "graph_editor" => {
                payload
                    .offscreen_viewports
                    .get_mut(&OffscreenViewport::GraphEditor)
                    .unwrap()
                    .show(ui, ui.available_size());
            }
            "inspector" => payload.inspector_tabs.ui(
                ui,
                payload.app_context.renderable_thing.as_ref(),
                &mut payload.graph_editor.editor_state,
                &mut payload.graph_editor.custom_state,
            ),
            _ => panic!("Invalid split name {name}"),
        }
    }
}
