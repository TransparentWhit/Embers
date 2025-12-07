use super::{Attributes, living_entity};
use crate::ui::world::{HotbarSelectionUpdated, PlayerCamera};
use crate::utils::NamespacedKey;
use crate::utils::input::{DoubleClicks, InputButton, just_pressed, pressed};
use crate::world::entity::living::attributes::embers;
use crate::world::item::{ItemActionTrigger, ItemStack};
use avian3d::prelude::*;
use bevy::input::mouse::{AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use bevy::time::Stopwatch;
use bevy::window::PrimaryWindow;
use bevy_tnua::prelude::*;
use std::collections::HashMap;
use std::iter::repeat;
use std::ops::DerefMut;
use std::sync::{LazyLock, RwLock};
use std::time::Duration;

macro_rules! controls {
    ($ident:ident, M@$default_mouse:ident) => {
        pub static $ident: RwLock<InputButton> =
            RwLock::new(InputButton::MouseButton(MouseButton::$default_mouse));
    };
    ($ident:ident, K@$default_key:ident) => {
        pub static $ident: RwLock<InputButton> =
            RwLock::new(InputButton::Keycode(KeyCode::$default_key));
    };
    [$ident:ident, $len:expr, $($default_type:ident@$default_value:ident),+ $(,)?] => {
        pub static $ident: [RwLock<InputButton>; $len] = [$(controls!(@parse $default_type@$default_value)),*];
    };
    (@parse M@$default_mouse:ident) => {
        RwLock::new(InputButton::MouseButton(MouseButton::$default_mouse))
    };
    (@parse K@$default_key:ident) => {
        RwLock::new(InputButton::Keycode(KeyCode::$default_key))
    };
}
controls!(CONTROLS_MOVEMENT, M@Left);
controls!(CONTROLS_INTERACT, K@KeyE);
controls!(CONTROLS_USE_MAIN_HAND, K@ShiftLeft);
controls!(CONTROLS_USE_OFF_HAND, M@Right);
controls!(CONTROLS_USE_ARMOR, K@KeyT);
controls![CONTROLS_HOTBARS, HOTBAR_SLOTS as usize,
    K@Digit1,
    K@Digit2,
    K@Digit3,
    K@Digit4,
    K@Digit5,
    K@Digit6,
];
controls!(CONTROLS_SWAP_OFF_HAND, K@KeyF);
controls!(CONTROLS_INVENTORY, K@KeyR);

pub type InventorySlot = i8;

pub(in crate::world) fn process_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
    double_clicks: Res<DoubleClicks>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut player: Single<
        (
            &Attributes,
            &PlayerInventory,
            &mut SelectedHotbarSlot,
            &mut PlayerActionStatus,
            &mut TnuaController,
        ),
        With<Player>,
    >,
    player_camera: Single<(&Camera, &PlayerCamera), With<PlayerCamera>>,
    mut hotbar_selection_updated_message: MessageWriter<HotbarSelectionUpdated>,
    time: Res<Time>,
) {
    let (attributes, inventory, selected_hotbar_slot, action_status, controller) =
        player.deref_mut();
    let prev_hotbar_selection = selected_hotbar_slot.0;
    for hotbar_slot in 0..HOTBAR_SLOTS {
        if just_pressed(&CONTROLS_HOTBARS[hotbar_slot as usize], &keys, &mouse) {
            selected_hotbar_slot.0 = hotbar_slot;
        }
    }
    if let MouseScrollUnit::Line = mouse_scroll.unit
        && let delta = mouse_scroll.delta.y as InventorySlot
        && delta != 0
    {
        selected_hotbar_slot.0 += delta;
        selected_hotbar_slot.0 = selected_hotbar_slot.0.rem_euclid(HOTBAR_SLOTS);
    }
    if prev_hotbar_selection != selected_hotbar_slot.0 {
        hotbar_selection_updated_message.write(HotbarSelectionUpdated);
    }
    let mut active_trigger = |slot| {
        let button = match slot {
            EquipmentSlot::MainHand => &CONTROLS_USE_MAIN_HAND,
            EquipmentSlot::OffHand => &CONTROLS_USE_OFF_HAND,
            EquipmentSlot::Armor => &CONTROLS_USE_ARMOR,
        };
        if double_clicks.double_clicked(*button.read().unwrap()) {
            Some(ItemActionTrigger::DoubleClick)
        } else if pressed(button, &keys, &mouse) {
            Some(ItemActionTrigger::Click)
        } else {
            None
        }
    };
    action_status.update_status(
        active_trigger(EquipmentSlot::MainHand),
        /*todo*/ false,
        active_trigger(EquipmentSlot::OffHand),
        active_trigger(EquipmentSlot::Armor),
    );
    action_status.tick(time.delta());
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
const FLOAT_HEIGHT: f32 = 0.85;

#[derive(Component)]
pub struct Player {
    pub flops: i32,
    pub hashes: i32,
    pub time_crystals: i32,
}

pub const HOTBAR_SLOTS: InventorySlot = 6;

#[derive(Component)]
pub struct PlayerInventory {
    pub items: [Option<ItemStack>; 38],
}
impl Default for PlayerInventory {
    fn default() -> Self {
        Self {
            items: [const { None }; 38],
        }
    }
}
/*
impl PlayerInventory {
    fn equipment_slot(equipment_slot: EquipmentSlot) -> InventorySlot {
        match equipment_slot {
            EquipmentSlot::MainHand => 0,
            EquipmentSlot::OffHand => 36,
            EquipmentSlot::Armor => 37,
        }
    }
}*/

#[derive(Component)]
pub struct SelectedHotbarSlot(pub InventorySlot);
impl Default for SelectedHotbarSlot {
    fn default() -> Self {
        Self(0)
    }
}

#[derive(Clone, Default)]
pub enum SlotActionStatus {
    #[default]
    Idle,
    Active {
        timer: Stopwatch,
        trigger: ItemActionTrigger,
    },
}

impl SlotActionStatus {
    pub fn idle() -> Self {
        Self::Idle
    }
    pub fn activate(trigger: ItemActionTrigger) -> Self {
        Self::Active {
            timer: Stopwatch::new(),
            trigger,
        }
    }
    #[inline]
    pub fn is_idle(&self) -> bool {
        matches!(self, Self::Idle)
    }
    #[inline]
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Active { .. })
    }
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub enum EquipmentSlot {
    MainHand,
    OffHand,
    Armor,
}

#[derive(Component)]
pub struct PlayerActionStatus(HashMap<EquipmentSlot, SlotActionStatus>);

impl PlayerActionStatus {
    fn new() -> Self {
        Self(
            [
                EquipmentSlot::MainHand,
                EquipmentSlot::OffHand,
                EquipmentSlot::Armor,
            ]
            .into_iter()
            .zip(repeat(SlotActionStatus::Idle))
            .collect(),
        )
    }
    fn update_status(
        &mut self,
        main_hand_trigger: Option<ItemActionTrigger>,
        main_single_wield: bool,
        off_hand_trigger: Option<ItemActionTrigger>,
        armor_trigger: Option<ItemActionTrigger>,
    ) {
        if self
            .update_slot_status(EquipmentSlot::MainHand, main_hand_trigger)
            .is_idle()
            || main_single_wield
        {
            self.update_slot_status(EquipmentSlot::OffHand, off_hand_trigger);
        }
        self.update_slot_status(EquipmentSlot::Armor, armor_trigger);
    }
    fn update_slot_status(
        &mut self,
        slot: EquipmentSlot,
        active_trigger: Option<ItemActionTrigger>,
    ) -> &SlotActionStatus {
        let status = self.0.get_mut(&slot).unwrap();
        if let Some(active_trigger) = active_trigger {
            if status.is_idle() {
                *status = SlotActionStatus::activate(active_trigger);
            }
        } else {
            if status.is_active() {
                *status = SlotActionStatus::idle();
            }
        }
        status
    }
    fn tick(&mut self, delta: Duration) {
        for (_, action_status) in &mut self.0 {
            if let SlotActionStatus::Active { timer, .. } = action_status {
                timer.tick(delta);
            }
        }
    }
}

pub fn player() -> impl Bundle {
    (
        living_entity(&ATTRIBUTES),
        Collider::cylinder(0.5, 1.7),
        PlayerInventory::default(),
        SelectedHotbarSlot::default(),
        PlayerActionStatus::new(),
        Player {
            flops: 0,
            hashes: 0,
            time_crystals: 0,
        },
    )
}
