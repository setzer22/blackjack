use super::*;

impl RootViewport {
    pub fn top_menubar(ui: &mut egui::Ui) {
        // When set, will load a new editor state at the end of this function
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Save As...").clicked() {
                    let file_location = rfd::FileDialog::new()
                        .set_file_name("Untitled.blj")
                        .add_filter("Blackjack Models", &["blj"])
                        .save_file();
                    if let Some(path) = file_location {
                        // TODO: Do not panic for this. Show error modal instead.
                        //serialization::save(state, ctx, path).expect("Serialization error");
                    }
                }
                if ui.button("Load").clicked() {
                    let file_location = rfd::FileDialog::new()
                        .add_filter("Blackjack Models", &["blj"])
                        .pick_file();
                    // TODO: Avoid panic
                    if let Some(path) = file_location {
                        //loaded_state =
                        //Some(serialization::load(ctx, path).expect("Deserialization error"));
                    }
                }
            });
        });
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
