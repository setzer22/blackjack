// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![forbid(unsafe_code)]

use std::ops::RangeInclusive;

use egui::{
    text::CCursor,
    text_edit::{CCursorRange, TextEditState},
    Memory, *,
};

// ----------------------------------------------------------------------------

/// Combined into one function (rather than two) to make it easier
/// for the borrow checker.
type GetSetValue<'a> = Box<dyn 'a + FnMut(Option<f64>) -> f64>;

fn get(get_set_value: &mut GetSetValue<'_>) -> f64 {
    (get_set_value)(None)
}

fn set(get_set_value: &mut GetSetValue<'_>, value: f64) {
    (get_set_value)(Some(value));
}

#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct SmartDragValue<'a, 'b> {
    get_set_value: GetSetValue<'a>,
    ranges: Ranges<'a, 'b>,
    speed: f64,
    prefix: String,
    suffix: String,
    soft_range: RangeInclusive<f64>,
    hard_range: RangeInclusive<f64>,
    decimals: usize,
    side: RangeSelectorSide,
    default_range_index: Option<usize>,
}

#[derive(Clone, Debug, Default)]
struct Ranges<'a, 'b> {
    speeds: &'a [f64],
    labels: &'a [&'b str],
}

impl<'a, 'b> Ranges<'a, 'b> {
    fn new(speeds: &'a [f64], labels: &'a [&'b str]) -> Self {
        assert_eq!(
            speeds.len(),
            labels.len(),
            "Should provide the same amount of speeds and labels"
        );
        assert!(speeds.len() > 1, "The smart range expects at least two different speeds to choose from. Use a regular DragValue instead.");
        Self { speeds, labels }
    }

    fn len(&self) -> usize {
        self.speeds.len()
    }
}

/// The global in-memory state of a [`SmartDragValue`] this is shared across all instances.
#[derive(Clone, Debug, Default)]
pub struct GlobalState {
    /// For temporary edit of a [`SmartDragValue`] value.
    edit_string: Option<String>,
}

/// The local in-memory state of a [`SmartDragValue`]. This is unique to each instance.
#[derive(Clone, Debug, Default)]
pub struct LocalState {
    /// Accumulated amount of mouse delta for the current drag event.
    drag_amount: f32,
    /// Accumulated amount of scroll wheel movement or trackpad finger scroll
    /// for the current drag event.
    scroll_amount: f32,
    /// The currently selected row for the scale selector.
    selected_row: Option<usize>,
    /// True when the current drag event started at the upper soft limit. This
    /// allows the slider to go past the soft max.
    upper_soft_limit: bool,
    /// True when the current drag event started at the bottom soft limit. This
    /// allows the slider to go past the soft min.
    lower_soft_limit: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RangeSelectorSide {
    Left,
    Right,
}

trait MemoryExt {
    /// Shorthand for 'SmartDragValue' global state
    fn sdv_global(&mut self) -> &mut GlobalState;

    /// Shorthand for 'SmartDragValue' local state.
    fn sdv_local(&mut self, id: Id) -> &mut LocalState;
}

impl MemoryExt for Memory {
    fn sdv_global(&mut self) -> &mut GlobalState {
        let id = Id::new("__smart_drag_value_state_id");
        self.data.get_temp_mut_or_default(id)
    }

    fn sdv_local(&mut self, id: Id) -> &mut LocalState {
        self.data.get_temp_mut_or_default(id)
    }
}

trait ColorExt {
    type Component;
    fn with_alpha(self, a: Self::Component) -> Self;
}

impl ColorExt for egui::Color32 {
    type Component = u8;

    fn with_alpha(self, a: Self::Component) -> Self {
        Color32::from_rgba_premultiplied(self.r(), self.g(), self.b(), a)
    }
}

impl<'a, 'b> SmartDragValue<'a, 'b> {
    pub fn new<Num: emath::Numeric>(
        value: &'a mut Num,
        speeds: &'a [f64],
        labels: &'a [&'b str],
    ) -> Self {
        let slf = Self::from_get_set(
            move |v: Option<f64>| {
                if let Some(v) = v {
                    *value = Num::from_f64(v);
                }
                value.to_f64()
            },
            speeds,
            labels,
        );

        if Num::INTEGRAL {
            slf.decimals(0)
                .clamp_range_hard(Num::MIN..=Num::MAX)
                .speed(0.25)
        } else {
            slf
        }
    }

    pub fn from_get_set(
        get_set_value: impl 'a + FnMut(Option<f64>) -> f64,
        speeds: &'a [f64],
        labels: &'a [&'b str],
    ) -> Self {
        Self {
            get_set_value: Box::new(get_set_value),
            speed: 1.0,
            prefix: Default::default(),
            suffix: Default::default(),
            soft_range: f64::NEG_INFINITY..=f64::INFINITY,
            hard_range: f64::NEG_INFINITY..=f64::INFINITY,
            decimals: 2,
            ranges: Ranges::new(speeds, labels),
            side: RangeSelectorSide::Right,
            default_range_index: None,
        }
    }

    /// How much the value changes when dragged one point (logical pixel).
    pub fn speed(mut self, speed: impl Into<f64>) -> Self {
        self.speed = speed.into();
        self
    }

    /// Clamp incoming and outgoing values to this range using a soft
    /// constraint. This means users won't be able to get past this value when
    /// dragging, unless the slider was already at the max / min value when the
    /// drag operation started.
    pub fn clamp_range_soft<Num: emath::Numeric>(
        mut self,
        hard_range: RangeInclusive<Num>,
    ) -> Self {
        self.soft_range = hard_range.start().to_f64()..=hard_range.end().to_f64();
        self
    }

    /// Clamp incoming and outgoing values to this range. Unlike the soft clamp
    /// range, there is no way for users to drag the slider past this value.
    /// Values set using other means like the TextEdit also can't get past this
    /// value.
    pub fn clamp_range_hard<Num: emath::Numeric>(
        mut self,
        hard_range: RangeInclusive<Num>,
    ) -> Self {
        self.hard_range = hard_range.start().to_f64()..=hard_range.end().to_f64();
        self
    }

    /// Show a prefix before the number, e.g. "x: "
    pub fn prefix(mut self, prefix: impl ToString) -> Self {
        self.prefix = prefix.to_string();
        self
    }

    /// Add a suffix to the number, this can be e.g. a unit ("°" or " m")
    pub fn suffix(mut self, suffix: impl ToString) -> Self {
        self.suffix = suffix.to_string();
        self
    }

    /// Set a number of decimals to display.
    /// Values will also be rounded to this number of decimals.
    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = decimals;
        self
    }

    /// Draw the range selector to the left of the DragValue
    pub fn left(mut self) -> Self {
        self.side = RangeSelectorSide::Left;
        self
    }

    /// Draw the range selector to the right of the DragValue
    pub fn right(mut self) -> Self {
        self.side = RangeSelectorSide::Right;
        self
    }

    /// Draw the range selector to the given `side` of the DragValue
    pub fn side(mut self, side: RangeSelectorSide) -> Self {
        self.side = side;
        self
    }

    /// The range selector will show the middle value by default. This setting
    /// can be used to override that and choose a different default index.
    pub fn default_range_index(mut self, index: usize) -> Self {
        self.default_range_index = Some(index);
        self
    }

    /// Returns: the selected speed multiplier
    fn draw_smart_range_selector(
        ui: &mut Ui,
        ranges: &Ranges,
        local_state: &mut LocalState,
        widget_rect: Rect,
        side: RangeSelectorSide,
    ) {
        let LocalState { selected_row, .. } = local_state;
        let selected_row = selected_row.expect("should be initialized");
        let padding = ui.spacing().button_padding;
        let size = ui.spacing().interact_size * 1.5;

        let top_left = match side {
            RangeSelectorSide::Left => {
                widget_rect.left_center()
                    - vec2(padding.x + size.x, size.y * (0.5 + selected_row as f32))
            }
            RangeSelectorSide::Right => {
                widget_rect.right_center() - vec2(-padding.x, size.y * (0.5 + selected_row as f32))
            }
        };

        let painter = ui.ctx().layer_painter(LayerId::new(
            Order::Tooltip,
            Id::new("smart_dragvalue_tooltip"),
        ));

        for (i, label) in ranges.labels.iter().enumerate() {
            let pos = top_left + vec2(0.0, size.y) * i as f32;

            painter.rect(
                Rect::from_min_size(pos, size),
                Rounding::none(),
                if selected_row == i {
                    ui.style().visuals.widgets.active.bg_fill.with_alpha(180)
                } else {
                    ui.style().noninteractive().bg_fill.with_alpha(180)
                },
                ui.style().noninteractive().bg_stroke,
            );

            painter.text(
                pos + vec2(size.x * 0.5, padding.y),
                Align2::CENTER_TOP,
                label,
                TextStyle::Body.resolve(ui.style()),
                ui.style().noninteractive().fg_stroke.color,
            );
        }

        #[cfg(target_os = "macos")]
        static CTRL_KEY_LABEL: &str = "↕ Cmd";
        #[cfg(not(target_os = "macos"))]
        static CTRL_KEY_LABEL: &str = "↕ Ctrl";

        let bottom_left = top_left + vec2(0.0, size.y * ranges.len() as f32);
        painter.text(
            bottom_left,
            Align2::LEFT_TOP,
            // Either Ctrl or Command, depending on platform
            CTRL_KEY_LABEL,
            TextStyle::Small.resolve(ui.style()),
            ui.style().noninteractive().fg_stroke.color,
        );
    }
}

impl<'a, 'b> Widget for SmartDragValue<'a, 'b> {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self {
            mut get_set_value,
            speed,
            soft_range,
            hard_range,
            prefix,
            suffix,
            decimals,
            ranges,
            side,
            default_range_index,
        } = self;

        let id = ui.next_auto_id();

        let shift = ui.input().modifiers.shift_only();
        let is_slow_speed = shift && ui.memory().is_being_dragged(id);

        let old_value = get(&mut get_set_value);
        let value = clamp_to_range(old_value, hard_range.clone());
        if old_value != value {
            set(&mut get_set_value, value);
        }
        let aim_rad = ui.input().aim_radius() as f64;

        let auto_decimals = (aim_rad / speed.abs()).log10().ceil().clamp(0.0, 15.0) as usize;
        let auto_decimals = auto_decimals + is_slow_speed as usize;

        let value_text = emath::format_with_decimals_in_range(value, decimals..=decimals);

        let kb_edit_id = id;
        let is_kb_editing = ui.memory().has_focus(kb_edit_id);

        let mut response = if is_kb_editing {
            let button_width = ui.spacing().interact_size.x;

            // This is none the first frame after the TextEdit has gained focus
            let select_all_text = ui.memory().sdv_global().edit_string.is_none();

            let mut value_text = ui
                .memory()
                .sdv_global()
                .edit_string
                .take()
                .unwrap_or(value_text);
            let response = ui.add(
                TextEdit::singleline(&mut value_text)
                    .id(kb_edit_id)
                    .desired_width(button_width)
                    .font(TextStyle::Monospace),
            );

            // We need to defer this operation after drawing the TextEdit for
            // the first time. Otherwise the memory won't have information for
            // this widget.
            if select_all_text {
                // Select the full text -- when users click they typically want
                // to replace the whole value. If not they can use the arrow keys
                if let Some(mut state) = TextEditState::load(ui.ctx(), kb_edit_id) {
                    state.set_ccursor_range(Some(CCursorRange::two(
                        CCursor::new(0),
                        CCursor::new(value_text.len()),
                    )));
                    state.store(ui.ctx(), kb_edit_id);
                }
            }

            if let Ok(parsed_value) = value_text.parse() {
                let parsed_value = clamp_to_range(parsed_value, hard_range.clone());
                set(&mut get_set_value, parsed_value);
            }
            if ui.input().key_pressed(Key::Enter) {
                ui.memory().surrender_focus(kb_edit_id);
                ui.memory().sdv_global().edit_string = None;
            } else {
                ui.memory().sdv_global().edit_string = Some(value_text);
            }
            response
        } else {
            let button =
                Button::new(RichText::new(format!("{prefix}{value_text}{suffix}")).monospace())
                    .wrap(false)
                    .sense(Sense::click_and_drag())
                    .min_size(ui.spacing().interact_size); // TODO: find some more generic solution to `min_size`

            let response = ui.add(button);
            let mut response = response.on_hover_cursor(CursorIcon::ResizeHorizontal);

            if ui.style().explanation_tooltips {
                response = response .on_hover_text(format!(
                    "{}{}{}\nDrag to edit or click to enter a value.\nPress 'Shift' while dragging for better control.",
                    prefix,
                    value as f32, // Show full precision value on-hover. TODO: figure out f64 vs f32
                    suffix
                ));
            }

            if response.clicked() {
                ui.memory().request_focus(kb_edit_id);
                ui.memory().sdv_global().edit_string = None; // Filled in next frame
            } else if response.dragged() {
                // We take the value and set back at the end of the scope.
                let mut local_state = std::mem::take(ui.memory().sdv_local(id));

                if response.drag_started() {
                    // NOTE: Only set the range if this is our first time
                    // editing this DragValue. Doing this remembers previous
                    // scale value from the last time the user touched this
                    // slider, which provides better UX: The range they picked
                    // was probably a good one.
                    if local_state.selected_row.is_none() {
                        local_state.selected_row =
                            Some(default_range_index.unwrap_or(ranges.len() / 2));
                    }
                    local_state.lower_soft_limit = value <= *soft_range.start();
                    local_state.upper_soft_limit = value >= *soft_range.end();
                }

                // Make sure the selected row always stays within bounds. This
                // could change if a different amount of range divisions is set
                // for this frame but old data was stored in memory.
                local_state.selected_row = local_state
                    .selected_row
                    .map(|s| s.clamp(0, ranges.len() - 1));

                ui.output().cursor_icon = CursorIcon::ResizeHorizontal;

                let mdelta = response.drag_delta();
                Self::draw_smart_range_selector(ui, &ranges, &mut local_state, response.rect, side);

                const MOUSE_AIM_PRECISION: f32 = 20.0;
                const SCROLL_WHEEL_PRECISION: f32 = 50.0;

                #[cfg(target_os = "macos")]
                let should_increment = !(ui.input().modifiers.command);
                #[cfg(not(target_os = "macos"))]
                let should_increment = !(ui.input().modifiers.ctrl);

                let delta_value = {
                    let LocalState {
                        ref mut drag_amount,
                        ref mut scroll_amount,
                        ref mut selected_row,
                        ..
                    } = local_state;

                    // Update the horizontal drag (increments / decrements)
                    *drag_amount += mdelta.x;
                    let discrete_increments = (*drag_amount / MOUSE_AIM_PRECISION).floor();
                    *drag_amount = drag_amount.rem_euclid(MOUSE_AIM_PRECISION);

                    // Update the scroll wheel
                    *scroll_amount += ui.input().scroll_delta.y;
                    if !should_increment {
                        *scroll_amount += mdelta.y;
                    }
                    let discrete_scrolls = (*scroll_amount / SCROLL_WHEEL_PRECISION).floor();
                    *scroll_amount = scroll_amount.rem_euclid(SCROLL_WHEEL_PRECISION);
                    let selected_row = selected_row.as_mut().expect("should be initialized");
                    *selected_row = (*selected_row as isize - discrete_scrolls as isize)
                        .clamp(0, ranges.len() as isize - 1)
                        as usize;

                    ranges.speeds[*selected_row] * discrete_increments as f64 * speed
                };
                let delta_value = if should_increment { delta_value } else { 0.0 };

                if delta_value != 0.0 {
                    let new_value = value + delta_value;
                    // Pick soft / hard bounds depending on when the drag event started.
                    let clamp_range = RangeInclusive::new(
                        if local_state.lower_soft_limit {
                            *hard_range.start()
                        } else {
                            *soft_range.start()
                        },
                        if local_state.upper_soft_limit {
                            *hard_range.end()
                        } else {
                            *soft_range.end()
                        },
                    );
                    let new_value = clamp_to_range(new_value, clamp_range);

                    set(&mut get_set_value, new_value);
                }

                *ui.memory().sdv_local(id) = local_state;
            } else if response.has_focus() {
                let change = ui.input().num_presses(Key::ArrowUp) as f64
                    + ui.input().num_presses(Key::ArrowRight) as f64
                    - ui.input().num_presses(Key::ArrowDown) as f64
                    - ui.input().num_presses(Key::ArrowLeft) as f64;

                if change != 0.0 {
                    let new_value = value + speed * change;
                    let new_value = emath::round_to_decimals(new_value, auto_decimals);
                    let new_value = clamp_to_range(new_value, hard_range.clone());
                    set(&mut get_set_value, new_value);
                }
            }

            response
        };

        // HACK: Sometimes the value can be out of range for a frame without
        // this. I don't know what causes it but this fixes it.
        let value = get(&mut get_set_value);
        set(&mut get_set_value, clamp_to_range(value, hard_range));

        response.changed = get(&mut get_set_value) != old_value;

        response.widget_info(|| WidgetInfo::drag_value(value));
        response
    }
}

fn clamp_to_range(x: f64, range: RangeInclusive<f64>) -> f64 {
    x.clamp(
        range.start().min(*range.end()),
        range.start().max(*range.end()),
    )
}
