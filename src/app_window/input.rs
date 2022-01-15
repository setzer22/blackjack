use winit::event::{ElementState, MouseButton, WindowEvent};

use crate::prelude::*;

#[derive(Default)]
pub struct InputSystem {
    pub mouse: MouseInput,
}

impl InputSystem {
    /// Called every frame, updates the input data structures
    pub fn update(&mut self) {
        self.mouse.update();
    }

    /// Called when a new `winit` window event is received.
    pub fn on_window_event(&mut self, event: &WindowEvent) {
        match event {
            // Cursor moved
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse
                    .on_cursor_move(Vec2::new(position.x as f32, position.y as f32));
            }

            WindowEvent::MouseWheel { delta, .. } => match delta {
                winit::event::MouseScrollDelta::LineDelta(_, y) => {
                    self.mouse.on_wheel_scroll(*y as f32);
                }
                winit::event::MouseScrollDelta::PixelDelta(pos) => {
                    self.mouse.on_wheel_scroll(pos.y as f32);
                }
            },

            WindowEvent::MouseInput { button, state, .. } => {
                self.mouse.on_button_event(*button, *state);
            }
            _ => {}
        }
    }
}

pub struct MouseInput {
    buttons: Input<MouseButton>,
    last_pos: Option<Vec2>,
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
            delta: Default::default(),
            wheel_delta: Default::default(),
        }
    }
}

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
