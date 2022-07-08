// Copyright (C) 2022 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::*;
use std::path::PathBuf;

pub enum AppRootAction {
    Save(PathBuf),
    Load(PathBuf),
    ExportJack(PathBuf),
    SetCodeViewerCode(String),
}

impl RootViewport {
    pub fn top_menubar(&mut self) -> Option<AppRootAction> {
        let mut action = None;
        egui::TopBottomPanel::top("top_menubar").show(&self.egui_context, |ui| {
            // When set, will load a new editor state at the end of this function
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Save As...").clicked() {
                        let file_location = rfd::FileDialog::new()
                            .set_file_name("Untitled.blj")
                            .add_filter("Blackjack Models", &["blj"])
                            .save_file();
                        if let Some(path) = file_location {
                            action = Some(AppRootAction::Save(path))
                        }
                    }
                    if ui.button("Load").clicked() {
                        let file_location = rfd::FileDialog::new()
                            .add_filter("Blackjack Models", &["blj"])
                            .pick_file();
                        if let Some(path) = file_location {
                            action = Some(AppRootAction::Load(path))
                        }
                    }
                    if ui.button("Export 'Jack'").clicked() {
                        let file_location = rfd::FileDialog::new()
                            .add_filter("Blackjack 'Jacks'", &["jack"])
                            .save_file();
                        if let Some(path) = file_location {
                            action = Some(AppRootAction::ExportJack(path))
                        }
                    }
                });
                ui.menu_button("Help", |ui| {
                    if ui.button("Diagnosics").clicked() {
                        self.diagnostics_open = true;
                    } else if ui.button("View graph source").clicked() {
                        self.code_viewer_open = true;
                    }
                });
            });
        });

        action
    }

    pub fn diagnostics_ui(&mut self) {
        egui::Window::new("Diagnostics")
            .open(&mut self.diagnostics_open)
            .show(&self.egui_context, |ui| {
                ui.label(format!("HiDPI scale: {}", ui.ctx().pixels_per_point()));
            });
    }

    pub fn code_viewer_ui(&mut self) {
        egui::Window::new("Code viewer")
            .open(&mut self.code_viewer_open)
            .show(&self.egui_context, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    code_viewer::code_view_ui(ui, self.code_viewer_code.as_deref().unwrap_or(""));
                });
            });
    }

    pub fn show_leaf(ui: &mut egui::Ui, payload: &mut Self, name: &str) {
        // TODO: These names here are hard-coded in the creation of the
        // SplitTree. We should be using some kind of identifier instead
        match name {
            "3d_view" => {
                payload.viewport_3d.show_ui(
                    ui,
                    payload
                        .offscreen_viewports
                        .get_mut(&OffscreenViewport::Viewport3d)
                        .unwrap(),
                    payload.app_context.mesh.as_ref(),
                );
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
                payload.app_context.mesh.as_ref(),
                &mut payload.graph_editor.state,
            ),
            _ => panic!("Invalid split name {}", name),
        }
    }
}
