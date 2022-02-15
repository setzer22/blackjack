use super::*;
use crate::prelude::*;
use egui::*;

#[cfg(target_arch = "wasm32")]
fn save_file() -> Option<std::path::PathBuf> {
    None
}

#[cfg(not(target_arch = "wasm32"))]
fn save_file() -> Option<std::path::PathBuf> {
    rfd::FileDialog::new().save_file()
}

impl InputParam {
    pub fn value_widget(&mut self, name: &str, ui: &mut Ui) {
        match &mut self.value {
            InputParamValue::Vector(vector) => {
                ui.label(name);

                ui.horizontal(|ui| {
                    ui.label("x");
                    ui.add(egui::DragValue::new(&mut vector.x).speed(0.1));
                    ui.label("y");
                    ui.add(egui::DragValue::new(&mut vector.y).speed(0.1));
                    ui.label("z");
                    ui.add(egui::DragValue::new(&mut vector.z).speed(0.1));
                });
            }
            InputParamValue::Scalar(scalar) => {
                let mut min = f32::NEG_INFINITY;
                let mut max = f32::INFINITY;
                for metadata in &self.metadata {
                    match metadata {
                        InputParamMetadata::MinMaxScalar {
                            min: min_val,
                            max: max_val,
                        } => {
                            min = *min_val;
                            max = *max_val;
                        }
                    }
                }
                ui.horizontal(|ui| {
                    ui.label(name);
                    ui.add(Slider::new(scalar, min..=max));
                });
            }
            InputParamValue::Selection { text, selection } => {
                if ui.text_edit_singleline(text).changed() {
                    *selection = text
                        .split(',')
                        .map(|x| {
                            x.parse::<u32>()
                                .map_err(|_| anyhow::anyhow!("Cannot parse number"))
                        })
                        .collect::<Result<Vec<_>>>()
                        .ok();
                }
            }
            InputParamValue::None => {
                ui.label(name);
            }
            InputParamValue::Enum { values, selection } => {
                let selected = if let Some(selection) = selection {
                    values[*selection as usize].clone()
                } else {
                    "".to_owned()
                };
                ComboBox::from_label(name)
                    .selected_text(selected)
                    .show_ui(ui, |ui| {
                        for (idx, value) in values.iter().enumerate() {
                            ui.selectable_value(selection, Some(idx as u32), value);
                        }
                    });
            }
            InputParamValue::NewFile { path } => {
                ui.label(name);
                ui.horizontal(|ui| {
                    if ui.button("Select").clicked() {
                        *path = save_file();
                    }
                    if let Some(ref path) = path {
                        ui.label(
                            path.clone()
                                .into_os_string()
                                .into_string()
                                .unwrap_or_else(|_| "<Invalid string>".to_owned()),
                        );
                    } else {
                        ui.label("No file selected");
                    }
                });
            }
        }
    }
}
