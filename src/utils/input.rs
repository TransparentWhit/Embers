use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::mouse::MouseButtonInput;
use bevy::prelude::*;
use bevy::time::Stopwatch;
use std::collections::{HashMap, HashSet};
use std::sync::RwLock;
use std::time::Duration;

macro_rules! button_input {
    ($event:ident) => {
        #[inline]
        pub fn $event(
            input_button: &RwLock<InputButton>,
            key_codes: &ButtonInput<KeyCode>,
            mouse_buttons: &ButtonInput<MouseButton>,
        ) -> bool {
            match *input_button.read().unwrap() {
                InputButton::Keycode(keycode) => key_codes.$event(keycode),
                InputButton::MouseButton(mouse_button) => mouse_buttons.$event(mouse_button),
            }
        }
    };
}

macro_rules! button_input_mut {
    ($event:ident) => {
        #[inline]
        pub fn $event(
            input_button: &RwLock<InputButton>,
            key_codes: &mut ButtonInput<KeyCode>,
            mouse_buttons: &mut ButtonInput<MouseButton>,
        ) -> bool {
            match *input_button.read().unwrap() {
                InputButton::Keycode(keycode) => key_codes.$event(keycode),
                InputButton::MouseButton(mouse_button) => mouse_buttons.$event(mouse_button),
            }
        }
    };
}

button_input!(pressed);
button_input!(just_pressed);
button_input_mut!(clear_just_pressed);
button_input!(just_released);
button_input_mut!(clear_just_released);

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub enum InputButton {
    Keycode(KeyCode),
    MouseButton(MouseButton),
}

#[derive(Resource)]
pub struct DoubleClicks {
    clicks: HashMap<InputButton, Stopwatch>,
    double_clicked: HashSet<InputButton>,
    just_started: HashSet<InputButton>,
    just_ended: HashSet<InputButton>,
}

impl Default for DoubleClicks {
    fn default() -> Self {
        Self {
            clicks: HashMap::new(),
            double_clicked: HashSet::new(),
            just_started: HashSet::with_capacity(1),
            just_ended: HashSet::with_capacity(1),
        }
    }
}

impl DoubleClicks {
    const THRESHOLD: Duration = Duration::from_millis(300);
    pub fn double_clicked(&self, input: InputButton) -> bool {
        self.double_clicked.contains(&input)
    }
    pub fn get_double_clicked(&self) -> impl ExactSizeIterator<Item = &InputButton> {
        self.double_clicked.iter()
    }
    pub fn any_double_clicked(&self, inputs: impl IntoIterator<Item = InputButton>) -> bool {
        inputs.into_iter().any(|input| self.double_clicked(input))
    }
    pub fn all_double_clicked(&self, inputs: impl IntoIterator<Item = InputButton>) -> bool {
        inputs.into_iter().all(|input| self.double_clicked(input))
    }
    fn release_all(&mut self) {
        self.just_ended.extend(self.double_clicked.drain());
    }
    pub fn just_started(&self, input: InputButton) -> bool {
        self.just_started.contains(&input)
    }
    pub fn get_just_started(&self) -> impl ExactSizeIterator<Item = &InputButton> {
        self.just_started.iter()
    }
    pub fn clear_just_started(&mut self, input: InputButton) -> bool {
        self.just_started.remove(&input)
    }
    pub fn any_just_started(&self, inputs: impl IntoIterator<Item = InputButton>) -> bool {
        inputs.into_iter().any(|input| self.just_started(input))
    }
    pub fn all_just_started(&self, inputs: impl IntoIterator<Item = InputButton>) -> bool {
        inputs.into_iter().all(|input| self.just_started(input))
    }
    pub fn just_ended(&self, input: InputButton) -> bool {
        self.just_ended.contains(&input)
    }
    pub fn get_just_ended(&self) -> impl ExactSizeIterator<Item = &InputButton> {
        self.just_ended.iter()
    }
    pub fn clear_just_ended(&mut self, input: InputButton) -> bool {
        self.just_ended.remove(&input)
    }
    pub fn any_just_ended(&self, inputs: impl IntoIterator<Item = InputButton>) -> bool {
        inputs.into_iter().any(|input| self.just_ended(input))
    }
    pub fn all_just_ended(&self, inputs: impl IntoIterator<Item = InputButton>) -> bool {
        inputs.into_iter().all(|input| self.just_ended(input))
    }
    fn clear(&mut self) {
        self.just_started.clear();
        self.just_ended.clear();
    }
    fn tick(&mut self, delta: Duration) {
        self.clicks.retain(|_, stopwatch| {
            stopwatch.tick(delta);
            stopwatch.elapsed() < Self::THRESHOLD
        });
    }
}

fn track_double_clicks(
    mut double_clicks: ResMut<DoubleClicks>,
    mut keyboard_inputs: MessageReader<KeyboardInput>,
    mut mouse_button_inputs: MessageReader<MouseButtonInput>,
    time: Res<Time>,
) {
    double_clicks.bypass_change_detection().clear();
    double_clicks.bypass_change_detection().tick(time.delta());
    let mut track_input = |button: InputButton, state: &ButtonState| match state {
        ButtonState::Pressed => match double_clicks.clicks.get_mut(&button) {
            Some(stopwatch) => {
                stopwatch.reset();
                double_clicks.just_started.insert(button);
                double_clicks.double_clicked.insert(button);
            }
            None => {
                double_clicks.clicks.insert(button, Stopwatch::new());
            }
        },
        ButtonState::Released => {
            double_clicks.double_clicked.remove(&button);
            double_clicks.just_ended.insert(button);
        }
    };
    for KeyboardInput {
        key_code, state, ..
    } in keyboard_inputs.read()
    {
        track_input(InputButton::Keycode(*key_code), state);
    }
    for MouseButtonInput { button, state, .. } in mouse_button_inputs.read() {
        track_input(InputButton::MouseButton(*button), state);
    }
}

pub(crate) fn input_plugin(app: &mut App) {
    app.add_systems(PreUpdate, track_double_clicks);
}
