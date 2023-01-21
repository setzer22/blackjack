// Copyright (C) 2023 setzer22 and contributors
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, ModifiersState, MouseButton, VirtualKeyCode, WindowEvent},
};

use crate::{egui_ext::RectUtils, prelude::*};

#[derive(Default)]
pub struct InputSystem {
    pub mouse: MouseInput,
    pub shift_down: bool,
    pub ctrl_down: bool,
    pub pressed: HashSet<VirtualKeyCode>,
}

/// Transforms a window-relative position `pos` into viewport relative
/// coordinates for a viewport at `viewport_rect`, with a `zoom_level` in a
/// window using a hiDPI scaling of `parent_scale`.
pub fn viewport_relative_position(
    position: PhysicalPosition<f64>,
    parent_scale: f32,
    viewport_rect: egui::Rect,
    zoom_level: f32,
) -> PhysicalPosition<f64> {
    let mut position = position;
    position.x -= (viewport_rect.min.x * parent_scale) as f64;
    position.y -= (viewport_rect.min.y * parent_scale) as f64;
    position.x *= zoom_level as f64;
    position.y *= zoom_level as f64;
    position
}

impl InputSystem {
    /// Called every frame, updates the input data structures
    pub fn update(&mut self) {
        self.mouse.update();
    }

    pub fn is_key_pressed(&self, key: VirtualKeyCode) -> bool {
        self.pressed.contains(&key)
    }

    /// Called when a new `winit` window event is received. The `viewport_rect`
    /// and `parent_scaling` are used to translate mouse events to
    /// viewport-relative coordinates
    pub fn on_window_event(
        &mut self,
        event: &WindowEvent,
        parent_scale: f32,
        viewport_rect: egui::Rect,
        mouse_captured_elsewhere: bool,
    ) {
        let mouse_in_viewport = !mouse_captured_elsewhere
            && self
                .mouse
                .last_pos_raw
                .map(|pos| {
                    viewport_rect
                        .scale_from_origin(parent_scale)
                        .contains(egui::pos2(pos.x, pos.y))
                })
                .unwrap_or(false);

        match event {
            // Cursor moves are always registered. The raw (untransformed) mouse
            // position is also stored so we know if the mosue is over the
            // viewport on the next events.
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse.last_pos_raw = Some(Vec2::new(position.x as f32, position.y as f32));

                let position = viewport_relative_position(
                    *position,
                    parent_scale,
                    viewport_rect,
                    1.0, // zoom doesn't affect cursor on this viewport
                );
                // We always update the raw mouse position, but the real mouse
                // position is not updated if the mouse is captured elsewhere.
                if !mouse_captured_elsewhere {
                    self.mouse
                        .on_cursor_move(Vec2::new(position.x as f32, position.y as f32));
                }
            }
            // Wheel events will only get registered when the cursor is inside the viewport
            WindowEvent::MouseWheel { delta, .. } if mouse_in_viewport => match delta {
                winit::event::MouseScrollDelta::LineDelta(_, y) => {
                    self.mouse.on_wheel_scroll(*y);
                }
                winit::event::MouseScrollDelta::PixelDelta(pos) => {
                    self.mouse.on_wheel_scroll(pos.y as f32);
                }
            },
            // Button events are a bit different: Presses can register inside
            // the viewport but releases will register anywhere.
            WindowEvent::MouseInput {
                button,
                state: state @ ElementState::Pressed,
                ..
            } if mouse_in_viewport => {
                self.mouse.on_button_event(*button, *state);
            }
            WindowEvent::MouseInput {
                button,
                state: state @ ElementState::Released,
                ..
            } => {
                self.mouse.on_button_event(*button, *state);
            }
            WindowEvent::ModifiersChanged(state) => {
                self.shift_down = state.contains(ModifiersState::SHIFT);
                self.ctrl_down = state.contains(ModifiersState::CTRL);
            }
            WindowEvent::KeyboardInput { input, .. } => {
                if let Some(key) = input.virtual_keycode {
                    match input.state {
                        ElementState::Pressed => {
                            self.pressed.insert(key);
                        }
                        ElementState::Released => {
                            self.pressed.remove(&key);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

pub struct MouseInput {
    buttons: Input<MouseButton>,
    last_pos: Option<Vec2>,
    last_pos_raw: Option<Vec2>,
    delta: Vec2,
    wheel_delta: f32,
}
impl MouseInput {
    pub fn on_cursor_move(&mut self, position: Vec2) {
        let last_pos = self.last_pos.unwrap_or(position);
        self.delta = position - last_pos;

        self.last_pos = Some(position);
    }

    pub fn on_button_event(&mut self, button: MouseButton, state: ElementState) {
        match state {
            ElementState::Pressed => self.buttons.press(button),
            ElementState::Released => self.buttons.release(button),
        };
    }

    pub fn on_wheel_scroll(&mut self, delta: f32) {
        self.wheel_delta = delta;
    }

    pub fn update(&mut self) {
        self.delta = Vec2::ZERO;
        self.wheel_delta = 0.0;
    }

    /// Get a reference to the mouse input's buttons.
    pub fn buttons(&self) -> &Input<MouseButton> {
        &self.buttons
    }

    /// Get a reference to the mouse input's last pos.
    pub fn position(&self) -> Option<Vec2> {
        self.last_pos
    }

    /// Get a reference to the mouse input's delta.
    pub fn cursor_delta(&self) -> Vec2 {
        self.delta
    }

    /// Get a reference to the mouse input's wheel delta.
    pub fn wheel_delta(&self) -> f32 {
        self.wheel_delta
    }
}
impl Default for MouseInput {
    fn default() -> Self {
        Self {
            buttons: Input::new(),
            last_pos: Default::default(),
            last_pos_raw: Default::default(),
            delta: Default::default(),
            wheel_delta: Default::default(),
        }
    }
}

#[derive(Default)]
pub struct Input<Button> {
    pressed: HashSet<Button>,
    just_pressed: HashSet<Button>,
    just_released: HashSet<Button>,
}

impl<Button> Input<Button>
where
    Button: Clone + Copy + PartialEq + Eq + std::hash::Hash,
{
    pub fn new() -> Self {
        Self {
            pressed: HashSet::new(),
            just_pressed: HashSet::new(),
            just_released: HashSet::new(),
        }
    }

    pub fn press(&mut self, button: Button) {
        self.pressed.insert(button);
        self.just_pressed.insert(button);
    }

    pub fn release(&mut self, button: Button) {
        self.pressed.remove(&button);
        self.just_released.insert(button);
    }

    pub fn pressed(&self, button: Button) -> bool {
        self.pressed.contains(&button)
    }

    pub fn just_pressed(&self, button: Button) -> bool {
        self.just_pressed.contains(&button)
    }

    pub fn just_released(&self, button: Button) -> bool {
        self.just_released.contains(&button)
    }

    pub fn update(&mut self) {
        self.just_pressed.clear();
        self.just_released.clear();
    }
}
