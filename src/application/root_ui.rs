use std::path::PathBuf;

use super::*;

pub enum AppRootAction {
    Save(PathBuf),
    Load(PathBuf),
}

impl RootViewport {
    pub fn top_menubar(ui: &mut egui::Ui) -> Option<AppRootAction> {
        let mut action = None;

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
            });
        });

        action
    }

    pub fn show_leaf(ui: &mut egui::Ui, payload: &mut Self, name: &str) {
        // TODO: These names here are hard-coded in the creation of the
        // SplitTree. We should be using some kind of identifier instead
        match name {
            "3d_view" => {
                payload
                    .offscreen_viewports
                    .get_mut(&OffscreenViewport::Viewport3d)
                    .unwrap()
                    .show(ui, ui.available_size());
            }
            "graph_editor" => {
                payload
                    .offscreen_viewports
                    .get_mut(&OffscreenViewport::GraphEditor)
                    .unwrap()
                    .show(ui, ui.available_size());
            }
            "inspector" => {
                ui.label("Properties inspector goes here");
            }
            _ => panic!("Invalid split name {}", name),
        }
    }
}
