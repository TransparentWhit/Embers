use super::{living_actor, Attributes};
use crate::dim::actor::living::attributes::embers;
use crate::dim::item::inventory::{
    Inventory, InventorySlot, ItemDestination, ItemMoveQuantity, ItemSource, MoveItemCommandExt,
};
use crate::dim::item::{HandActionWield, ItemAction, ItemActionTrigger, ItemActionWield};
use crate::dim::item::{ItemActionSlot, ItemActions};
use crate::input::{just_pressed, pressed, DoubleClicks, InputButton};
use crate::ui::dim::{HotbarSelectionUpdated, PlayerCamera};
use crate::utils::NamespacedKey;
use crate::GameState;
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
use std::ops::Range;
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

fn process_input_item_actions(
    spatial_query: SpatialQuery,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    double_clicks: Res<DoubleClicks>,
    mut player: Single<
        (
            &PlayerInventory,
            &SelectedHotbarSlot,
            &mut PlayerActionStatus,
            &Transform,
        ),
        With<Player>,
    >,
    mut item_actions: Query<&mut ItemActions>,
    time: Res<Time>,
) {
    let (ref inventory, ref selected_hotbar_slot, ref mut action_status, ref transform) = *player;
    let active_item_trigger = |control: &RwLock<InputButton>,
                               double_clicks: &DoubleClicks,
                               keys: &ButtonInput<KeyCode>,
                               mouse: &ButtonInput<MouseButton>|
     -> Option<ItemActionTrigger> {
        if double_clicks.double_clicked(*control.read().unwrap()) {
            Some(ItemActionTrigger::DoubleClick)
        } else if pressed(control, keys, mouse) {
            Some(ItemActionTrigger::Click)
        } else {
            None
        }
    };
    let update_slot_action = |action_status: &mut PlayerActionStatus,
                              slot: EquipmentSlot,
                              trigger: Option<ItemActionTrigger>,
                              item_action: Option<&mut ItemAction>| {
        let status = action_status.0.get_mut(&slot).unwrap();
        match trigger {
            Some(trigger) => {
                if status.is_idle() {
                    let can_activate = match slot {
                        EquipmentSlot::MainHand | EquipmentSlot::OffHand => item_action
                            .as_ref()
                            .map(|action| action.trigger == trigger)
                            .unwrap_or(true),
                        EquipmentSlot::Armor => true,
                    };
                    if can_activate {
                        *status = SlotActionStatus::activate(trigger);
                        if let Some(mut action) = item_action {
                            (action.on_begin)((&spatial_query, transform));
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
                    if let Some(mut action) = item_action {
                        (action.on_end)((&spatial_query, transform), None); // todo
                    }
                    *status = SlotActionStatus::idle();
                }
            }
        }
    };
    let main_hand_action = inventory[PlayerInventory::MAIN_HAND_SLOT]
        .and_then(|entity| item_actions.get_mut(entity).ok())
        .and_then(|actions| actions.into_inner().get_mut(ItemActionSlot::Hands));
    let main_hand_single_wield = matches!(
        main_hand_action,
        Some(ItemAction {
            wield: ItemActionWield::Hands(HandActionWield::Single),
            ..
        })
    );
    update_slot_action(
        action_status,
        EquipmentSlot::MainHand,
        active_item_trigger(&CONTROLS_USE_MAIN_HAND, &double_clicks, &keys, &mouse),
        main_hand_action,
    );
    let main_hand_active = action_status.0[&EquipmentSlot::MainHand].is_active();
    let off_hand_action = inventory[selected_hotbar_slot.0]
        .and_then(|entity| item_actions.get_mut(entity).ok())
        .and_then(|actions| actions.into_inner().get_mut(ItemActionSlot::Hands));
    update_slot_action(
        action_status,
        EquipmentSlot::OffHand,
        if main_hand_single_wield
            && matches!(
                off_hand_action,
                Some(ItemAction {
                    wield: ItemActionWield::Hands(HandActionWield::Single),
                    ..
                })
            )
            && main_hand_active
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
}

pub fn process_input_hotbar(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
    mut player: Single<(Entity, &PlayerInventory, &mut SelectedHotbarSlot), With<Player>>,
    mut hotbar_selection_updated_message: MessageWriter<HotbarSelectionUpdated>,
) {
    let (player, ref inventory, ref mut selected_hotbar_slot) = *player;
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
    if just_pressed(&CONTROLS_SWAP_OFF_HAND, &keys, &mouse) {
        commands.move_item(
            ItemSource::inventory_slot(player, PlayerInventory::MAIN_HAND_SLOT, inventory),
            ItemDestination::inventory_slot(player, selected_hotbar_slot.0, inventory),
            ItemMoveQuantity::All,
        );
    }
}

fn process_input_movement(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut player: Single<(&Attributes, &mut TnuaController), With<Player>>,
    player_camera: Single<(&Camera, &PlayerCamera), With<PlayerCamera>>,
) {
    let (ref attributes, ref mut controller) = *player;
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
    const HOTBAR_SLOTS: Range<InventorySlot> = 0..HOTBAR_SLOTS;
    const ARMOR_SLOT: InventorySlot = 36;
    const MAIN_HAND_SLOT: InventorySlot = 37;
    pub fn armor(&self) -> Option<Entity> {
        self[Self::ARMOR_SLOT]
    }
    pub fn main_hand(&self) -> Option<Entity> {
        self[Self::MAIN_HAND_SLOT]
    }
    pub fn hotbar(&self, slot: InventorySlot) -> Option<Entity> {
        debug_assert!(
            Self::HOTBAR_SLOTS.contains(&slot),
            "Slot out of bounds for player hotbar: {}",
            slot
        );
        self[slot]
    }
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
        living_actor(&ATTRIBUTES),
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

pub(in crate::dim) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            process_input_item_actions,
            process_input_hotbar,
            process_input_movement,
        )
            .run_if(in_state(GameState::Dimension)),
    );
}
