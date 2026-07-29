//! Input state: keyboard keys, mouse buttons, mouse wheel, cursor
//! position. Collected from `winit` events by the engine; consumed
//! read-only by gameplay code and scripts.

use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u16),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyState {
    Pressed,
    Released,
}

#[derive(Default, Debug)]
pub struct Input {
    pub mouse_pos: (f32, f32),
    pub mouse_delta: (f32, f32),
    pub wheel: f32,
    keys_down: HashSet<u32>,
    keys_pressed_this_frame: HashSet<u32>,
    mouse_down: HashSet<MouseButton>,
    mouse_pressed_this_frame: HashSet<MouseButton>,
}

impl Input {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed a key event. `scancode` is the physical key (not the
    /// localized logical key) so controls are layout-independent.
    pub fn on_key(&mut self, scancode: u32, state: KeyState) {
        match state {
            KeyState::Pressed => {
                if self.keys_down.insert(scancode) {
                    self.keys_pressed_this_frame.insert(scancode);
                }
            }
            KeyState::Released => {
                self.keys_down.remove(&scancode);
            }
        }
    }

    pub fn on_mouse_button(&mut self, btn: MouseButton, state: KeyState) {
        match state {
            KeyState::Pressed => {
                if self.mouse_down.insert(btn) {
                    self.mouse_pressed_this_frame.insert(btn);
                }
            }
            KeyState::Released => {
                self.mouse_down.remove(&btn);
            }
        }
    }

    pub fn on_cursor(&mut self, x: f32, y: f32) {
        let prev = self.mouse_pos;
        self.mouse_delta = (x - prev.0, y - prev.1);
        self.mouse_pos = (x, y);
    }

    pub fn on_wheel(&mut self, delta: f32) {
        self.wheel += delta;
    }

    /// Is the key currently held down?
    pub fn key(&self, scancode: u32) -> bool {
        self.keys_down.contains(&scancode)
    }

    /// Was the key pressed this frame (edge-triggered)?
    pub fn key_pressed(&self, scancode: u32) -> bool {
        self.keys_pressed_this_frame.contains(&scancode)
    }

    pub fn mouse(&self, btn: MouseButton) -> bool {
        self.mouse_down.contains(&btn)
    }

    pub fn mouse_pressed(&self, btn: MouseButton) -> bool {
        self.mouse_pressed_this_frame.contains(&btn)
    }

    /// Call at the end of the frame to clear per-frame edge state.
    pub fn end_frame(&mut self) {
        self.keys_pressed_this_frame.clear();
        self.mouse_pressed_this_frame.clear();
        self.mouse_delta = (0.0, 0.0);
        self.wheel = 0.0;
    }
}
