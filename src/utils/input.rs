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
    app.insert_resource(DoubleClicks::default())
        .add_systems(PreUpdate, track_double_clicks);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Copy, Clone, Eq, PartialEq, Hash)]
    enum DummyInput {
        Input1,
        Input2,
    }

    impl From<DummyInput> for InputButton {
        fn from(input: DummyInput) -> Self {
            match input {
                DummyInput::Input1 => InputButton::Keycode(KeyCode::KeyQ),
                DummyInput::Input2 => InputButton::MouseButton(MouseButton::Left),
            }
        }
    }

    #[test]
    fn test_double_clicked() {
        let mut double_clicks = DoubleClicks::default();
        let input = DummyInput::Input1.into();
        double_clicks.clicks.insert(input, Stopwatch::new());
        double_clicks.clicks.get_mut(&input).unwrap().reset();
        double_clicks.double_clicked.insert(input);
        assert!(double_clicks.double_clicked(input));
        assert!(!double_clicks.double_clicked(DummyInput::Input2.into()));
    }

    #[test]
    fn getting_double_clicked() {
        let mut double_clicks = DoubleClicks::default();
        let input1 = DummyInput::Input1.into();
        let input2 = DummyInput::Input2.into();
        double_clicks.double_clicked.insert(input1);
        double_clicks.double_clicked.insert(input2);
        let double_clicked = double_clicks.get_double_clicked();
        assert_eq!(double_clicked.len(), 2);
        for clicked_input in double_clicked {
            assert!(double_clicks.double_clicked.contains(clicked_input));
        }
    }

    #[test]
    fn test_any_double_clicked() {
        let mut double_clicks = DoubleClicks::default();
        let input1 = DummyInput::Input1.into();
        let input2 = DummyInput::Input2.into();
        double_clicks.double_clicked.insert(input1);
        assert!(double_clicks.any_double_clicked([input1]));
        assert!(!double_clicks.any_double_clicked([input2]));
        assert!(double_clicks.any_double_clicked([input1, input2]));
    }

    #[test]
    fn test_all_double_clicked() {
        let mut double_clicks = DoubleClicks::default();
        let input1 = DummyInput::Input1.into();
        let input2 = DummyInput::Input2.into();
        double_clicks.double_clicked.insert(input1);
        assert!(double_clicks.all_double_clicked([input1]));
        assert!(!double_clicks.all_double_clicked([input1, input2]));
        double_clicks.double_clicked.insert(input2);
        assert!(double_clicks.all_double_clicked([input1, input2]));
    }

    #[test]
    fn test_just_started() {
        let mut double_clicks = DoubleClicks::default();
        let input = DummyInput::Input1.into();
        assert!(!double_clicks.just_started(input));
        double_clicks.just_started.insert(input);
        assert!(double_clicks.just_started(input));
    }

    #[test]
    fn getting_just_started() {
        let mut double_clicks = DoubleClicks::default();
        let input1 = DummyInput::Input1.into();
        let input2 = DummyInput::Input2.into();
        double_clicks.just_started.insert(input1);
        double_clicks.just_started.insert(input2);
        let just_started = double_clicks.get_just_started();
        assert_eq!(just_started.len(), 2);
        for started_input in just_started {
            assert!(double_clicks.just_started.contains(started_input));
        }
    }

    #[test]
    fn clearing_just_started() {
        let mut double_clicks = DoubleClicks::default();
        let input = DummyInput::Input1.into();
        double_clicks.just_started.insert(input);
        assert!(double_clicks.just_started(input));
        assert!(double_clicks.clear_just_started(input));
        assert!(!double_clicks.just_started(input));
        assert!(!double_clicks.clear_just_started(input));
    }

    #[test]
    fn test_any_just_started() {
        let mut double_clicks = DoubleClicks::default();
        let input1 = DummyInput::Input1.into();
        let input2 = DummyInput::Input2.into();
        assert!(!double_clicks.any_just_started([input1]));
        assert!(!double_clicks.any_just_started([input2]));
        assert!(!double_clicks.any_just_started([input1, input2]));
        double_clicks.just_started.insert(input1);
        assert!(double_clicks.any_just_started([input1]));
        assert!(!double_clicks.any_just_started([input2]));
        assert!(double_clicks.any_just_started([input1, input2]));
    }

    #[test]
    fn test_all_just_started() {
        let mut double_clicks = DoubleClicks::default();
        let input1 = DummyInput::Input1.into();
        let input2 = DummyInput::Input2.into();
        assert!(!double_clicks.all_just_started([input1]));
        assert!(!double_clicks.all_just_started([input2]));
        assert!(!double_clicks.all_just_started([input1, input2]));
        double_clicks.just_started.insert(input1);
        assert!(double_clicks.all_just_started([input1]));
        assert!(!double_clicks.all_just_started([input1, input2]));
        double_clicks.just_started.insert(input2);
        assert!(double_clicks.all_just_started([input1, input2]));
    }

    #[test]
    fn test_just_ended() {
        let mut double_clicks = DoubleClicks::default();
        let input = DummyInput::Input1.into();
        assert!(!double_clicks.just_ended(input));
        double_clicks.just_ended.insert(input);
        assert!(double_clicks.just_ended(input));
    }

    #[test]
    fn getting_just_ended() {
        let mut double_clicks = DoubleClicks::default();
        let input1 = DummyInput::Input1.into();
        let input2 = DummyInput::Input2.into();
        double_clicks.just_ended.insert(input1);
        double_clicks.just_ended.insert(input2);
        let just_ended = double_clicks.get_just_ended();
        assert_eq!(just_ended.len(), 2);
        for ended_input in just_ended {
            assert!(double_clicks.just_ended.contains(ended_input));
        }
    }

    #[test]
    fn clearing_just_ended() {
        let mut double_clicks = DoubleClicks::default();
        let input = DummyInput::Input1.into();
        double_clicks.just_ended.insert(input);
        assert!(double_clicks.just_ended(input));
        assert!(double_clicks.clear_just_ended(input));
        assert!(!double_clicks.just_ended(input));
        assert!(!double_clicks.clear_just_ended(input));
    }

    #[test]
    fn test_any_just_ended() {
        let mut double_clicks = DoubleClicks::default();
        let input1 = DummyInput::Input1.into();
        let input2 = DummyInput::Input2.into();
        assert!(!double_clicks.any_just_ended([input1]));
        assert!(!double_clicks.any_just_ended([input2]));
        assert!(!double_clicks.any_just_ended([input1, input2]));
        double_clicks.just_ended.insert(input1);
        assert!(double_clicks.any_just_ended([input1]));
        assert!(!double_clicks.any_just_ended([input2]));
        assert!(double_clicks.any_just_ended([input1, input2]));
    }

    #[test]
    fn test_all_just_ended() {
        let mut double_clicks = DoubleClicks::default();
        let input1 = DummyInput::Input1.into();
        let input2 = DummyInput::Input2.into();
        assert!(!double_clicks.all_just_ended([input1]));
        assert!(!double_clicks.all_just_ended([input2]));
        assert!(!double_clicks.all_just_ended([input1, input2]));
        double_clicks.just_ended.insert(input1);
        assert!(double_clicks.all_just_ended([input1]));
        assert!(!double_clicks.all_just_ended([input1, input2]));
        double_clicks.just_ended.insert(input2);
        assert!(double_clicks.all_just_ended([input1, input2]));
    }

    #[test]
    fn releasing_all() {
        let mut double_clicks = DoubleClicks::default();
        let input1 = DummyInput::Input1.into();
        let input2 = DummyInput::Input2.into();
        double_clicks.double_clicked.insert(input1);
        double_clicks.double_clicked.insert(input2);
        double_clicks.release_all();
        assert!(double_clicks.double_clicked.is_empty());
        assert!(double_clicks.just_ended.contains(&input1));
        assert!(double_clicks.just_ended.contains(&input2));
    }

    #[test]
    fn clearing() {
        let mut double_clicks = DoubleClicks::default();
        let input1 = DummyInput::Input1.into();
        let input2 = DummyInput::Input2.into();
        double_clicks.just_started.insert(input1);
        double_clicks.just_ended.insert(input2);
        double_clicks.double_clicked.insert(input1);
        double_clicks.clear();
        assert!(double_clicks.just_started.is_empty());
        assert!(double_clicks.just_ended.is_empty());
        assert!(double_clicks.double_clicked.contains(&input1));
    }

    #[test]
    fn ticking() {
        let mut double_clicks = DoubleClicks::default();
        let input = DummyInput::Input1.into();
        let mut stopwatch = Stopwatch::new();
        stopwatch.tick(Duration::from_millis(100));
        double_clicks.clicks.insert(input, stopwatch);
        double_clicks.tick(Duration::from_millis(150));
        assert!(double_clicks.clicks.contains_key(&input));
        double_clicks.tick(Duration::from_millis(200));
        assert!(!double_clicks.clicks.contains_key(&input));
    }

    #[test]
    fn general_double_clicking() {
        let mut double_clicks = DoubleClicks::default();
        let input1 = DummyInput::Input1.into();
        let input2 = DummyInput::Input2.into();
        double_clicks.clicks.insert(input1, Stopwatch::new());
        double_clicks.clicks.get_mut(&input1).unwrap().reset();
        double_clicks.double_clicked.insert(input1);
        double_clicks.just_started.insert(input1);
        assert!(double_clicks.double_clicked(input1));
        assert!(double_clicks.just_started(input1));
        assert!(!double_clicks.just_ended(input1));
        double_clicks.clear();
        assert!(!double_clicks.just_started(input1));
        assert!(double_clicks.double_clicked(input1));
        double_clicks.double_clicked.remove(&input1);
        double_clicks.just_ended.insert(input1);
        assert!(!double_clicks.double_clicked(input1));
        assert!(double_clicks.just_ended(input1));
        double_clicks.clear();
        assert!(!double_clicks.just_ended(input1));
        double_clicks.double_clicked.insert(input1);
        double_clicks.double_clicked.insert(input2);
        double_clicks.just_started.insert(input1);
        double_clicks.just_started.insert(input2);
        assert!(double_clicks.all_double_clicked([input1, input2]));
        assert!(double_clicks.all_just_started([input1, input2]));
        double_clicks.clear_just_started(input1);
        assert!(!double_clicks.just_started(input1));
        assert!(double_clicks.just_started(input2));
        double_clicks.release_all();
        assert!(double_clicks.double_clicked.is_empty());
        assert!(double_clicks.just_ended.contains(&input1));
        assert!(double_clicks.just_ended.contains(&input2));
    }
}
