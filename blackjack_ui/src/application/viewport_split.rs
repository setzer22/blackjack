// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use egui::*;

#[derive(Clone)]
pub enum ViewportSplitKind {
    Horizontal,
    Vertical,
}

#[derive(Clone)]
pub struct ViewportSplit {
    // The fraction of the first element of the split. The second element will
    // fill all available size
    pub fraction: f32,
    pub kind: ViewportSplitKind,
    pub separator_width: f32,
}

impl ViewportSplit {
    pub fn vertical(fraction: f32) -> Self {
        Self {
            fraction,
            separator_width: 10.0,
            kind: ViewportSplitKind::Vertical,
        }
    }
    pub fn horizontal(fraction: f32) -> Self {
        Self {
            fraction,
            separator_width: 10.0,
            kind: ViewportSplitKind::Horizontal,
        }
    }
}

impl ViewportSplit {
    pub fn show<Payload: PayloadTrait>(
        &mut self,
        ui: &mut Ui,
        payload: &mut Payload,
        view_1: impl FnOnce(&mut Ui, &mut Payload),
        view_2: impl FnOnce(&mut Ui, &mut Payload),
    ) {
        // HACK: Sometimes, in some window managers / platforms, windows can
        // end up having zero size. This can lead to NaNs getting their way
        // into the `fraction` and breaking the UI. This workaround solves it.
        if self.fraction.is_nan() {
            self.fraction = 0.5;
        }

        let total_space = ui.available_rect_before_wrap();
        let hsep = self.separator_width * 0.5;

        let response = match self.kind {
            ViewportSplitKind::Horizontal => {
                let width_1 = total_space.width() * self.fraction;
                let width_2 = total_space.width() * (1.0 - self.fraction);

                let mut rect1 = total_space;
                rect1.set_right(total_space.right() - (width_2 + hsep));

                let mut rect2 = total_space;
                rect2.set_left(total_space.left() + (width_1 + hsep));

                ui.horizontal(|ui| {
                    view_1(&mut ui.child_ui(rect1, Layout::default()), payload);
                    view_2(&mut ui.child_ui(rect2, Layout::default()), payload);
                });

                let separator_rect = Rect::from_min_max(rect1.right_top(), rect2.left_bottom());

                let resp = ui.allocate_rect(separator_rect, Sense::drag());

                let painter = ui.painter();
                painter.line_segment(
                    [separator_rect.center_top(), separator_rect.center_bottom()],
                    Stroke {
                        width: 2.0,
                        color: if resp.hovered() {
                            Color32::WHITE
                        } else {
                            Color32::DARK_GRAY
                        },
                    },
                );

                self.fraction = (self.fraction + resp.drag_delta().x / total_space.width())
                    // Clamp fraction so that it never becomes zero
                    .clamp(0.05, 0.95);

                resp
            }
            ViewportSplitKind::Vertical => {
                // @CopyPaste TODO: Get rid of this code duplication

                let height_1 = total_space.height() * self.fraction;
                let height_2 = total_space.height() * (1.0 - self.fraction);

                let mut rect1 = total_space;
                rect1.set_bottom(total_space.bottom() - (height_2 + hsep));

                let mut rect2 = total_space;
                rect2.set_top(total_space.top() + (height_1 + hsep));

                ui.horizontal(|ui| {
                    view_1(&mut ui.child_ui(rect1, Layout::default()), payload);
                    view_2(&mut ui.child_ui(rect2, Layout::default()), payload);
                });

                let separator_rect = Rect::from_min_max(rect1.left_bottom(), rect2.right_top());

                let resp = ui.allocate_rect(separator_rect, Sense::drag());

                let painter = ui.painter();
                painter.line_segment(
                    [separator_rect.left_center(), separator_rect.right_center()],
                    Stroke {
                        width: 2.0,
                        color: if resp.hovered() {
                            Color32::WHITE
                        } else {
                            Color32::DARK_GRAY
                        },
                    },
                );

                self.fraction = (self.fraction + resp.drag_delta().y / total_space.height())
                    // Clamp fraction so that it never becomes zero
                    .clamp(0.05, 0.95);

                resp
            }
        };

        // We need to notify the payload (really, the app root) that this widget
        // is being interacted with so it can stop propagating mouse events to
        // the child viewports.
        if response.dragged() {
            payload.notify_interacted()
        }
    }
}

#[derive(Clone)]
pub enum SplitTree {
    Leaf(String),
    Split {
        left: Box<SplitTree>,
        right: Box<SplitTree>,
        split: ViewportSplit,
    },
}

pub trait PayloadTrait {
    /// Called when the split tree is currently being interacted with. This is
    /// used to notify this fact back to the caller.
    fn notify_interacted(&mut self);
}

impl SplitTree {
    pub fn show<Payload: PayloadTrait>(
        &mut self,
        ui: &mut Ui,
        payload: &mut Payload,
        show_leaf: fn(&mut Ui, state: &mut Payload, &str) -> (),
    ) {
        match self {
            SplitTree::Leaf(ref name) => show_leaf(ui, payload, name),
            SplitTree::Split { left, right, split } => split.show(
                ui,
                payload,
                |ui, state| left.show(ui, state, show_leaf),
                |ui, state| right.show(ui, state, show_leaf),
            ),
        }
    }

    pub fn default_tree() -> SplitTree {
        SplitTree::Split {
            left: Box::new(SplitTree::Split {
                left: Box::new(SplitTree::Leaf("3d_view".into())),
                right: Box::new(SplitTree::Leaf("inspector".into())),
                split: ViewportSplit::horizontal(0.66),
            }),
            right: Box::new(SplitTree::Leaf("graph_editor".into())),
            split: ViewportSplit::vertical(0.5),
        }
    }
}
