use super::*;

/// The widget value trait is used to determine how to display each [`ValueType`]
impl WidgetValueTrait for ValueType {
    fn value_widget(&mut self, param_name: &str, ui: &mut egui::Ui) {
        match self {
            ValueType::Vector(vector) => {
                ui.label(param_name);

                ui.horizontal(|ui| {
                    ui.label("x");
                    ui.add(egui::DragValue::new(&mut vector.x).speed(0.1));
                    ui.label("y");
                    ui.add(egui::DragValue::new(&mut vector.y).speed(0.1));
                    ui.label("z");
                    ui.add(egui::DragValue::new(&mut vector.z).speed(0.1));
                });
            }
            ValueType::Scalar { value, min, max } => {
                ui.horizontal(|ui| {
                    ui.label(param_name);
                    ui.add(egui::Slider::new(value, *min..=*max));
                });
            }
            ValueType::Selection { text, selection } => {
                if ui.text_edit_singleline(text).changed() {
                    *selection = SelectionExpression::parse(text).ok();
                }
            }
            ValueType::None => {
                ui.label(param_name);
            }
            ValueType::Enum {
                values,
                selected: selection,
            } => {
                let selected = if let Some(selection) = selection {
                    values[*selection as usize].clone()
                } else {
                    "".to_owned()
                };
                egui::ComboBox::from_label(param_name)
                    .selected_text(selected)
                    .show_ui(ui, |ui| {
                        for (idx, value) in values.iter().enumerate() {
                            ui.selectable_value(selection, Some(idx as u32), value);
                        }
                    });
            }
            ValueType::NewFile { path } => {
                ui.label(param_name);
                ui.horizontal(|ui| {
                    if ui.button("Select").clicked() {
                        *path = rfd::FileDialog::new().save_file();
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
