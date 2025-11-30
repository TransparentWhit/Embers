use super::{Attributes, living_entity};
use crate::ui::world::PlayerCamera;
use crate::utils::NamespacedKey;
use crate::world::entity::living::attributes::embers;
use crate::world::item::ItemActionTrigger;
use avian3d::prelude::*;
use bevy::input::mouse::{AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_tnua::prelude::*;
use std::collections::HashMap;
use std::ops::DerefMut;
use std::sync::{LazyLock, RwLock};

macro_rules! controls {
    ($ident:ident, M@$default_mouse:ident) => {
        pub static $ident: RwLock<InputButton> =
            RwLock::new(InputButton::MouseButton(MouseButton::$default_mouse));
    };
    ($ident:ident, K@$default_key:ident) => {
        pub static $ident: RwLock<InputButton> =
            RwLock::new(InputButton::Keycode(KeyCode::$default_key));
    };
}
controls!(CONTROLS_MOVEMENT, M@Left);
controls!(CONTROLS_INTERACT, K@KeyE);
controls!(CONTROLS_USE_MAIN_HAND, K@ShiftLeft);
controls!(CONTROLS_USE_OFF_HAND, M@Right);
controls!(CONTROLS_HOTBAR_0, K@Digit1);
controls!(CONTROLS_HOTBAR_1, K@Digit2);
controls!(CONTROLS_HOTBAR_2, K@Digit3);
controls!(CONTROLS_HOTBAR_3, K@Digit4);
controls!(CONTROLS_HOTBAR_4, K@Digit5);
controls!(CONTROLS_HOTBAR_5, K@Digit6);
controls!(CONTROLS_SWAP_OFF_HAND, K@KeyF);
controls!(CONTROLS_INVENTORY, K@KeyR);

pub enum InputButton {
    Keycode(KeyCode),
    MouseButton(MouseButton),
}

macro_rules! button_input {
    ($event:ident) => {
        #[inline]
        fn $event(
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
button_input!(pressed);
button_input!(just_pressed);
button_input!(just_released);

pub(in crate::world) fn process_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut player: Single<(&Attributes, &mut SelectedHotbarSlot, &mut TnuaController), With<Player>>,
    player_camera: Single<(&Camera, &PlayerCamera), With<PlayerCamera>>,
) {
    let (attributes, selected_item_slot, controller) = player.deref_mut();
    match () {
        _ if just_pressed(&CONTROLS_HOTBAR_0, &keys, &mouse) => selected_item_slot.0 = 0,
        _ if just_pressed(&CONTROLS_HOTBAR_1, &keys, &mouse) => selected_item_slot.0 = 1,
        _ if just_pressed(&CONTROLS_HOTBAR_2, &keys, &mouse) => selected_item_slot.0 = 2,
        _ if just_pressed(&CONTROLS_HOTBAR_3, &keys, &mouse) => selected_item_slot.0 = 3,
        _ if just_pressed(&CONTROLS_HOTBAR_4, &keys, &mouse) => selected_item_slot.0 = 4,
        _ if just_pressed(&CONTROLS_HOTBAR_5, &keys, &mouse) => selected_item_slot.0 = 5,
        _ => {}
    }
    if let MouseScrollUnit::Pixel = mouse_scroll.unit {
        selected_item_slot.0 += mouse_scroll.delta.y as u8;
        selected_item_slot.0 %= HOTBAR_SLOTS;
    }
    if just_pressed(&CONTROLS_SWAP_OFF_HAND, &keys, &mouse) {
        // todo
    };
    if pressed(&CONTROLS_USE_MAIN_HAND, &keys, &mouse) {}
    if pressed(&CONTROLS_MOVEMENT, &keys, &mouse)
        && let Some(physical_cursor_position) = window.physical_cursor_position()
    {
        let (camera, camera_config) = *player_camera;
        let forward = match camera_config {
            PlayerCamera::Isometric {
                distance,
                height,
                angle,
            } => {
                let viewport = camera.viewport.clone().unwrap_or_default();
                let normalized = (physical_cursor_position - viewport.physical_position.as_vec2())
                    / viewport.physical_size.as_vec2();
                let forward = Vec3::new(-distance * angle.cos(), -height, -distance * angle.sin())
                    .normalize();
                let right = forward.cross(Vec3::Y).normalize();
                Dir3::new(
                    ((right * (normalized.x * 2. - 1.))
                        + (Vec3::Y.cross(right) * (1. - normalized.y * 2.)))
                        .with_y(0.),
                )
                .ok()
            }
        };
        controller.basis(TnuaBuiltinWalk {
            float_height: FLOAT_HEIGHT,
            desired_velocity: forward
                .map(|direction| {
                    direction.as_vec3() * attributes.0[&embers::MOVEMENT_SPEED].value()
                })
                .unwrap_or_default(),
            desired_forward: forward,
            ..default()
        });
    }
}

static ATTRIBUTES: LazyLock<HashMap<NamespacedKey, f32>> = LazyLock::new(|| {
    HashMap::from([
        (embers::MAX_HEALTH.clone(), 20.),
        (embers::MOVEMENT_SPEED.clone(), 2.),
    ])
});
static HITBOX: LazyLock<Collider> = LazyLock::new(|| Collider::cylinder(0.5, 1.7));
const FLOAT_HEIGHT: f32 = 0.85;

#[derive(Component)]
pub struct Player {
    pub flops: i32,
    pub hashes: i32,
    pub time_crystals: i32,
}

pub const HOTBAR_SLOTS: u8 = 6;

#[derive(Component)]
pub struct SelectedHotbarSlot(u8);
impl Default for SelectedHotbarSlot {
    fn default() -> Self {
        Self(0)
    }
}

#[derive(Default)]
pub enum SlotActionStatus {
    #[default]
    Idle,
    Active {
        started: f32,
        trigger: ItemActionTrigger,
    },
}

#[derive(Eq, Hash, PartialEq)]
pub enum EquipmentSlot {
    MainHand,
    OffHand,
    Armor,
}

#[derive(Component)]
pub struct ActionStatus(HashMap<EquipmentSlot, SlotActionStatus>);
impl Default for ActionStatus {
    fn default() -> Self {
        Self(HashMap::from([(
            EquipmentSlot::MainHand,
            SlotActionStatus::Idle,
        )]))
    }
}

pub fn player() -> impl Bundle {
    (
        living_entity(&ATTRIBUTES),
        HITBOX.clone(),
        SelectedHotbarSlot::default(),
        Player {
            flops: 0,
            hashes: 0,
            time_crystals: 0,
        },
    )
}
