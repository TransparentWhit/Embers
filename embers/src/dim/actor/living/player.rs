use super::{Attributes, living_actor};
use crate::GameState;
use crate::dim::actor::living::attributes::embers;
use crate::dim::item::inventory::{
    Inventory, InventorySlot, ItemDestination, ItemMoveQuantity, ItemSource, MoveItemCommandExt,
};
use crate::dim::item::{
    HandActionWield, ItemActionEnvironment, ItemActionTrigger, ItemActionWield, SlotItemActions,
};
use crate::dim::item::{ItemActionSlot, ItemActions};
use crate::input::{DoubleClicks, InputButton, just_pressed, pressed};
use crate::ui::dim::PlayerCamera;
use crate::utils::NamespacedKey;
use avian3d::prelude::*;
use bevy::input::mouse::{AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use bevy::time::Stopwatch;
use bevy::window::PrimaryWindow;
use bevy_tnua::builtins::TnuaBuiltinDash;
use bevy_tnua::prelude::*;
use std::collections::HashMap;
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
    mut commands: Commands,
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
    hotbar_selection_updated_reader: MessageReader<HotbarSelectionUpdated>,
    off_hand_swapped_reader: MessageReader<OffHandSwapped>,
    time: Res<Time>,
) {
    let (inventory, selected_hotbar_slot, ref mut action_status, transform) = *player;
    let mut environment: ItemActionEnvironment = (&mut commands, &spatial_query, transform);
    let mut update_slot_action =
        |equipment_slot: EquipmentSlot, control: &RwLock<InputButton>, slot: Option<Entity>| {
            let mut trigger = if double_clicks.double_clicked(*control.read().unwrap()) {
                Some(ItemActionTrigger::DoubleClick)
            } else if pressed(control, &keys, &mouse) {
                Some(ItemActionTrigger::Click)
            } else {
                None
            };
            if matches!(equipment_slot, EquipmentSlot::OffHand)
                && match *action_status.get(EquipmentSlot::MainHand) {
                    SlotActionStatus::Idle => false,
                    SlotActionStatus::Active {
                        trigger: current_main_hand_trigger,
                        ..
                    } => inventory
                        .main_hand()
                        .and_then(|entity| item_actions.get(entity).ok())
                        .and_then(|actions| {
                            actions
                                .get(ItemActionSlot::Hands)
                                .get(current_main_hand_trigger)
                        })
                        .map(|action| match action.wield {
                            ItemActionWield::Armor => unreachable!(),
                            ItemActionWield::Hands(HandActionWield::Single) => slot
                                .and_then(|entity| item_actions.get(entity).ok())
                                .map(|actions| actions.get(equipment_slot.item_action_slot()))
                                .zip(trigger)
                                .and_then(|(actions, trigger)| actions.get(trigger))
                                .map(|action| {
                                    matches!(
                                        action.wield,
                                        ItemActionWield::Hands(HandActionWield::Dual)
                                    )
                                })
                                .unwrap_or(false),
                            ItemActionWield::Hands(HandActionWield::Dual) => true,
                        })
                        .unwrap_or(false),
                }
                || !hotbar_selection_updated_reader.is_empty()
            {
                trigger.take();
            }
            if matches!(
                equipment_slot,
                EquipmentSlot::MainHand | EquipmentSlot::OffHand
            ) && !off_hand_swapped_reader.is_empty()
            {
                trigger.take();
            }
            let mut binding = SlotItemActions::default();
            let item_action = slot
                .and_then(|entity| item_actions.get_mut(entity).ok())
                .map(|actions| {
                    actions
                        .into_inner()
                        .get_mut(equipment_slot.item_action_slot())
                })
                .unwrap_or(&mut binding);
            let status = action_status.get_mut(equipment_slot);
            match trigger {
                Some(active_trigger) => {
                    if status.is_idle() {
                        if let Some(action) = item_action.get_mut(active_trigger) {
                            *status = SlotActionStatus::activate(active_trigger);
                            (action.on_begin)(&mut environment);
                        }
                    } else if let SlotActionStatus::Active {
                        timer,
                        trigger: current_trigger,
                    } = status
                    {
                        if let Some(action) = item_action.get_mut(active_trigger) {
                            let finished = timer.elapsed() >= action.duration;
                            if finished || active_trigger != *current_trigger {
                                (action.on_end)(
                                    &mut environment,
                                    if finished {
                                        None
                                    } else {
                                        Some(timer.elapsed())
                                    },
                                );
                                *status = SlotActionStatus::activate(active_trigger);
                                (action.on_begin)(&mut environment);
                            }
                        } else if item_action.get(*current_trigger).is_none() {
                            *status = SlotActionStatus::idle();
                        }
                    }
                }
                None => {
                    if let SlotActionStatus::Active { timer, trigger } = status {
                        if let Some(action) = item_action.get_mut(*trigger) {
                            (action.on_end)(
                                &mut environment,
                                Some(timer.elapsed()).take_if(|used| action.duration >= *used),
                            );
                        }
                        *status = SlotActionStatus::idle();
                    }
                }
            }
        };
    update_slot_action(
        EquipmentSlot::MainHand,
        &CONTROLS_USE_MAIN_HAND,
        inventory.main_hand(),
    );
    update_slot_action(
        EquipmentSlot::OffHand,
        &CONTROLS_USE_OFF_HAND,
        inventory.hotbar(selected_hotbar_slot.0),
    );
    update_slot_action(EquipmentSlot::Armor, &CONTROLS_USE_ARMOR, inventory.armor());
    action_status.tick(time.delta());
}

#[derive(Message)]
pub struct HotbarSelectionUpdated;

#[derive(Message)]
pub struct OffHandSwapped;

pub fn process_input_hotbar(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
    mut player: Single<(Entity, &PlayerInventory, &mut SelectedHotbarSlot), With<Player>>,
    mut hotbar_selection_updated_writer: MessageWriter<HotbarSelectionUpdated>,
    mut off_hand_swapped_writer: MessageWriter<OffHandSwapped>,
) {
    let (player, inventory, ref mut selected_hotbar_slot) = *player;
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
        hotbar_selection_updated_writer.write(HotbarSelectionUpdated);
    }
    if just_pressed(&CONTROLS_SWAP_OFF_HAND, &keys, &mouse) {
        commands.move_item(
            ItemSource::inventory_slot(player, PlayerInventory::MAIN_HAND_SLOT, inventory),
            ItemDestination::inventory_slot(player, selected_hotbar_slot.0, inventory),
            ItemMoveQuantity::All,
        );
        off_hand_swapped_writer.write(OffHandSwapped);
    }
}

fn process_input_movement(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut player: Single<(&Attributes, &mut TnuaController), With<Player>>,
    player_camera: Single<(&Camera, &PlayerCamera), With<PlayerCamera>>,
) {
    let (attributes, ref mut controller) = *player;
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

impl EquipmentSlot {
    fn item_action_slot(&self) -> ItemActionSlot {
        match self {
            EquipmentSlot::MainHand | EquipmentSlot::OffHand => ItemActionSlot::Hands,
            EquipmentSlot::Armor => ItemActionSlot::Armor,
        }
    }
}

#[derive(Component, Debug)]
pub struct PlayerActionStatus(
    /// Main hand
    SlotActionStatus,
    /// Off hand
    SlotActionStatus,
    /// Armor
    SlotActionStatus,
);

impl PlayerActionStatus {
    fn new() -> Self {
        Self(
            SlotActionStatus::Idle,
            SlotActionStatus::Idle,
            SlotActionStatus::Idle,
        )
    }
    fn get(&self, slot: EquipmentSlot) -> &SlotActionStatus {
        match slot {
            EquipmentSlot::MainHand => &self.0,
            EquipmentSlot::OffHand => &self.1,
            EquipmentSlot::Armor => &self.2,
        }
    }
    fn get_mut(&mut self, slot: EquipmentSlot) -> &mut SlotActionStatus {
        match slot {
            EquipmentSlot::MainHand => &mut self.0,
            EquipmentSlot::OffHand => &mut self.1,
            EquipmentSlot::Armor => &mut self.2,
        }
    }
    fn tick(&mut self, delta: Duration) {
        for action_status in [&mut self.0, &mut self.1, &mut self.2] {
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
    app.add_message::<HotbarSelectionUpdated>();
    app.add_message::<OffHandSwapped>();
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
