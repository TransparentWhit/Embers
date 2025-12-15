use super::{Attributes, living_entity};
use crate::ui::world::{HotbarSelectionUpdated, PlayerCamera};
use crate::utils::NamespacedKey;
use crate::utils::input::{DoubleClicks, InputButton, just_pressed, pressed};
use crate::world::entity::living::attributes::embers;
use crate::world::item::inventory::{Inventory, InventorySlot};
use crate::world::item::{HandActionWield, ItemAction, ItemActionTrigger, ItemActionWield};
use crate::world::item::{ItemActionSlot, ItemActions};
use avian3d::prelude::*;
use bevy::input::mouse::{AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use bevy::time::Stopwatch;
use bevy::window::PrimaryWindow;
use bevy_tnua::builtins::TnuaBuiltinDash;
use bevy_tnua::prelude::*;
use std::collections::HashMap;
use std::iter::repeat;
use std::marker::PhantomData;
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
controls!(CONTROLS_ROLL, K@Space);
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
    item_actions: Query<&ItemActions>,
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

    #[inline]
    fn active_item_trigger(
        control: &RwLock<InputButton>,
        double_clicks: &DoubleClicks,
        keys: &ButtonInput<KeyCode>,
        mouse: &ButtonInput<MouseButton>,
    ) -> Option<ItemActionTrigger> {
        if double_clicks.double_clicked(*control.read().unwrap()) {
            Some(ItemActionTrigger::DoubleClick)
        } else if pressed(control, keys, mouse) {
            Some(ItemActionTrigger::Click)
        } else {
            None
        }
    }
    #[inline]
    fn update_slot_action(
        action_status: &mut PlayerActionStatus,
        slot: EquipmentSlot,
        trigger: Option<ItemActionTrigger>,
        item_action: Option<&ItemAction>,
    ) {
        let status = action_status.0.get_mut(&slot).unwrap();
        match trigger {
            Some(trigger) => {
                if status.is_idle() {
                    let can_activate = match slot {
                        EquipmentSlot::MainHand | EquipmentSlot::OffHand => item_action
                            .map(|action| action.trigger == trigger)
                            .unwrap_or(true),
                        EquipmentSlot::Armor => true,
                    };
                    if can_activate {
                        *status = SlotActionStatus::activate(trigger);
                        if let Some(action) = item_action {
                            (action.on_begin)();
                        }
                    }
                } else if let SlotActionStatus::Active {
                    trigger: current_trigger,
                    ..
                } = status
                {
                    if *current_trigger != trigger {
                        *status = SlotActionStatus::activate(trigger);
                    }
                }
            }
            None => {
                if status.is_active() {
                    if let Some(action) = item_action {
                        (action.on_end)();
                    }
                    *status = SlotActionStatus::idle();
                }
            }
        }
    }
    let main_hand_action = inventory[PlayerInventory::MAIN_HAND_SLOT]
        .as_ref()
        .and_then(|entity| item_actions.get(*entity).ok())
        .and_then(|actions| actions.get(ItemActionSlot::Hands));
    let off_hand_action = inventory[selected_hotbar_slot.0]
        .as_ref()
        .and_then(|entity| item_actions.get(*entity).ok())
        .and_then(|actions| actions.get(ItemActionSlot::Hands));
    update_slot_action(
        action_status,
        EquipmentSlot::MainHand,
        active_item_trigger(&CONTROLS_USE_MAIN_HAND, &double_clicks, &keys, &mouse),
        main_hand_action,
    );
    let main_hand_active = action_status.0[&EquipmentSlot::MainHand].is_active();
    update_slot_action(
        action_status,
        EquipmentSlot::OffHand,
        if matches!(
            main_hand_action,
            Some(ItemAction {
                wield: ItemActionWield::Hands(HandActionWield::Single),
                ..
            })
        ) && matches!(
            off_hand_action,
            Some(ItemAction {
                wield: ItemActionWield::Hands(HandActionWield::Single),
                ..
            })
        ) && main_hand_active
        {
            active_item_trigger(&CONTROLS_USE_OFF_HAND, &double_clicks, &keys, &mouse)
        } else {
            None
        },
        off_hand_action,
    );
    update_slot_action(
        action_status,
        EquipmentSlot::Armor,
        active_item_trigger(&CONTROLS_USE_ARMOR, &double_clicks, &keys, &mouse),
        None,
    );
    action_status.tick(time.delta());
    let forward = if pressed(&CONTROLS_MOVEMENT, &keys, &mouse)
        && let Some(physical_cursor_position) = window.physical_cursor_position()
    {
        let (camera, camera_config) = *player_camera;
        match camera_config {
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
        }
    } else {
        None
    };
    controller.basis(TnuaBuiltinWalk {
        float_height: FLOAT_HEIGHT,
        desired_velocity: forward
            .map(|direction| direction.as_vec3() * attributes.0[&embers::MOVEMENT_SPEED].value())
            .unwrap_or_default(),
        desired_forward: forward,
        ..default()
    });
    if pressed(&CONTROLS_ROLL, &keys, &mouse) {
        controller.action(TnuaBuiltinDash {
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

#[derive(Component, Debug)]
pub struct Player {
    pub flops: i32,
    pub hashes: i32,
    pub time_crystals: i32,
}

pub const HOTBAR_SLOTS: InventorySlot = 6;

pub type PlayerInventory = Inventory<38, PhantomData<Player>>;

impl PlayerInventory {
    const ARMOR_SLOT: InventorySlot = 36;
    const MAIN_HAND_SLOT: InventorySlot = 37;
}

#[derive(Component, Debug)]
pub struct SelectedHotbarSlot(pub InventorySlot);

impl Default for SelectedHotbarSlot {
    fn default() -> Self {
        Self(0)
    }
}

#[derive(Clone, Debug, Default)]
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

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EquipmentSlot {
    MainHand,
    OffHand,
    Armor,
}

#[derive(Component, Debug)]
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
    fn tick(&mut self, delta: Duration) {
        for action_status in self.0.values_mut() {
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
        PlayerInventory::new(),
        SelectedHotbarSlot::default(),
        PlayerActionStatus::new(),
        Player {
            flops: 0,
            hashes: 0,
            time_crystals: 0,
        },
    )
}
