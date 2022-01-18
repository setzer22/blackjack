use egui::*;

pub enum ViewportSplitKind {
    Horizontal,
    Vertical,
}

pub struct ViewportSplit {
    // The size of the first element of the split. The second element will fill all available size
    pub size: f32,
}

impl ViewportSplit {
    pub fn show(
        &mut self,
        &ui: &mut Ui,
        ui_top: impl FnOnce(&mut Ui) -> (),
        ui_bottom: impl FnOnce(&mut Ui) -> (),
    ) {
        let total_space = 
    }
}
