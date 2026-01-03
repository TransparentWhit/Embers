use super::{AttributeBase, Attributes, living_actor};
use crate::GameState;
use crate::dim::actor::living::attributes::embers;
use crate::dim::item::inv::{
    Inventory, InventorySlot, ItemDestination, ItemMoveQuantity, ItemSource, MoveItemCommandExt,
};
use crate::dim::item::{
    HandActionWield, ItemAction, ItemActionEnvironment, ItemActionTrigger, ItemActionWield,
    ItemStack, SlotItemActions,
};
use crate::dim::item::{InitialItemActions, ItemActionSlot};
use crate::input::{DoubleClicks, InputButton, just_pressed, pressed};
use crate::reg::{OrRegistry, Reg, RegistryAccess};
use crate::ui::dim::PlayerCamera;
use crate::utils::{Keyed, NamespacedKey};
use avian3d::prelude::*;
use bevy::input::mouse::{AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use bevy::time::Stopwatch;
use bevy::window::PrimaryWindow;
use bevy_tnua::builtins::TnuaBuiltinDash;
use bevy_tnua::prelude::*;
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
    mut environment: ItemActionEnvironment,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    double_clicks: Res<DoubleClicks>,
    mut player: Single<(&mut PlayerActionStatus, &mut PlayerEquipmentActions), With<Player>>,
    item_action_reg: Reg<ItemAction>,
    hotbar_selection_updated_reader: MessageReader<HotbarSelectionUpdated>,
    off_hand_swapped_reader: MessageReader<OffHandSwapped>,
    time: Res<Time>,
) {
    let (ref mut action_status, ref mut equipment_actions) = *player;
    let mut update_slot_action = |equipment_slot: EquipmentSlot, control: &RwLock<InputButton>| {
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
                } => equipment_actions
                    .main_hand()
                    .get(current_main_hand_trigger)
                    .map(|main_hand_action| match main_hand_action.wield {
                        ItemActionWield::Armor => unreachable!(),
                        ItemActionWield::Hands(HandActionWield::Single) => trigger
                            .and_then(|trigger| equipment_actions.off_hand().get(trigger))
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
        let slot_actions = equipment_actions.get_slot_mut(equipment_slot);
        let status = action_status.get_mut(equipment_slot);
        match trigger {
            Some(active_trigger) => {
                if status.is_idle() {
                    if let Some(action) = slot_actions.get(active_trigger) {
                        *status = SlotActionStatus::activate(active_trigger);
                        (action.on_begin)(&mut environment);
                    }
                } else if let SlotActionStatus::Active {
                    ref timer,
                    trigger: current_trigger,
                } = *status
                {
                    if let Some(action) = slot_actions.get_mut(active_trigger) {
                        let finished = timer.elapsed() >= action.duration;
                        if finished || active_trigger != current_trigger {
                            (action.on_end)(
                                &mut environment,
                                if finished {
                                    None
                                } else {
                                    Some(timer.elapsed())
                                },
                            )
                            .and_then(|new_action| item_action_reg.get(&new_action))
                            .inspect(|new_action| {
                                *action = (*new_action).clone();
                            });
                            *status = SlotActionStatus::activate(active_trigger);
                            (action.on_begin)(&mut environment);
                        }
                    } else if slot_actions.get(current_trigger).is_none() {
                        *status = SlotActionStatus::idle();
                    }
                }
            }
            None => {
                if let SlotActionStatus::Active { ref timer, trigger } = *status {
                    if let Some(action) = slot_actions.get_mut(trigger) {
                        (action.on_end)(
                            &mut environment,
                            Some(timer.elapsed()).take_if(|used| action.duration >= *used),
                        )
                        .and_then(|new_action| item_action_reg.get(&new_action))
                        .inspect(|new_action| {
                            *action = (*new_action).clone();
                        });
                    }
                    *status = SlotActionStatus::idle();
                }
            }
        }
    };
    update_slot_action(EquipmentSlot::MainHand, &CONTROLS_USE_MAIN_HAND);
    update_slot_action(EquipmentSlot::OffHand, &CONTROLS_USE_OFF_HAND);
    update_slot_action(EquipmentSlot::Armor, &CONTROLS_USE_ARMOR);
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
    mut player: Single<
        (
            Entity,
            &PlayerInventory,
            &mut SelectedHotbarSlot,
            &mut PlayerEquipmentActions,
        ),
        With<Player>,
    >,
    item_action_reg: Reg<ItemAction>,
    initial_item_actions: Query<(&ItemStack, Option<&InitialItemActions>)>,
    initial_item_actions_reg: Reg<InitialItemActions>,
    mut hotbar_selection_updated_writer: MessageWriter<HotbarSelectionUpdated>,
    mut off_hand_swapped_writer: MessageWriter<OffHandSwapped>,
) {
    let (player, inventory, ref mut selected_hotbar_slot, ref mut equipment_actions) = *player;
    let prev_hotbar_selection = selected_hotbar_slot.0;
    let mut update_equipment_slot_actions =
        |equipment_slot: EquipmentSlot, slot: Option<Entity>| {
            let slot_actions = equipment_actions.get_slot_mut(equipment_slot);
            *slot_actions = SlotItemActions::default();
            slot.and_then(|item| initial_item_actions.get(item).ok())
                .and_then(|(item_stack, initial_item_actions)| {
                    initial_item_actions
                        .cloned()
                        .or_registry(&initial_item_actions_reg, item_stack.key())
                })
                .inspect(|initial_actions| {
                    let item_action_slot = equipment_slot.item_action_slot();
                    for trigger in [ItemActionTrigger::DoubleClick, ItemActionTrigger::Click] {
                        initial_actions
                            .get(item_action_slot, trigger)
                            .and_then(|action| item_action_reg.get(action))
                            .inspect(|action| slot_actions.set(trigger, (*action).clone()));
                    }
                });
        };
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
        update_equipment_slot_actions(
            EquipmentSlot::OffHand,
            inventory.hotbar(selected_hotbar_slot.0),
        );
        hotbar_selection_updated_writer.write(HotbarSelectionUpdated);
    }
    if just_pressed(&CONTROLS_SWAP_OFF_HAND, &keys, &mouse) {
        commands.move_item(
            ItemSource::inventory_slot(player, PlayerInventory::MAIN_HAND_SLOT, inventory),
            ItemDestination::inventory_slot(player, selected_hotbar_slot.0, inventory),
            ItemMoveQuantity::All,
        );
        update_equipment_slot_actions(EquipmentSlot::MainHand, inventory.main_hand());
        update_equipment_slot_actions(
            EquipmentSlot::OffHand,
            inventory.hotbar(selected_hotbar_slot.0),
        );
        off_hand_swapped_writer.write(OffHandSwapped);
    }
    update_equipment_slot_actions(EquipmentSlot::Armor, inventory.armor());
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
                let inward = Vec3::new(-distance * angle.cos(), -height, -distance * angle.sin())
                    .normalize();
                let right = inward.cross(Vec3::Y).normalize();
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

pub static KEY: LazyLock<NamespacedKey> = LazyLock::new(|| NamespacedKey::new_embers("player"));

const FLOAT_HEIGHT: f32 = 1.0;

#[derive(Component, Debug)]
#[require(
    SelectedHotbarSlot,
    PlayerActionStatus,
    PlayerEquipmentActions,
    PlayerInventory::new()
)]
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

#[derive(Component, Debug, Default)]
pub struct PlayerActionStatus(
    /// Main hand
    SlotActionStatus,
    /// Off hand
    SlotActionStatus,
    /// Armor
    SlotActionStatus,
);

impl PlayerActionStatus {
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

#[derive(Component, Default)]
pub struct PlayerEquipmentActions {
    main_hand: SlotItemActions,
    off_hand: SlotItemActions,
    armor: SlotItemActions,
}

impl PlayerEquipmentActions {
    pub fn get_slot(&self, slot: EquipmentSlot) -> &SlotItemActions {
        match slot {
            EquipmentSlot::MainHand => &self.main_hand,
            EquipmentSlot::OffHand => &self.off_hand,
            EquipmentSlot::Armor => &self.armor,
        }
    }
    pub fn get_slot_mut(&mut self, slot: EquipmentSlot) -> &mut SlotItemActions {
        match slot {
            EquipmentSlot::MainHand => &mut self.main_hand,
            EquipmentSlot::OffHand => &mut self.off_hand,
            EquipmentSlot::Armor => &mut self.armor,
        }
    }
    pub fn main_hand(&self) -> &SlotItemActions {
        &self.main_hand
    }
    pub fn off_hand(&self) -> &SlotItemActions {
        &self.off_hand
    }
    pub fn armor(&self) -> &SlotItemActions {
        &self.armor
    }
    pub fn clear_slot(&mut self, slot: EquipmentSlot) {
        *self.get_slot_mut(slot) = SlotItemActions::default();
    }
}

pub fn player(attribute_bases: impl RegistryAccess<Item = AttributeBase>) -> impl Bundle {
    (
        living_actor(&KEY, attribute_bases),
        Collider::cylinder(0.5, 1.7),
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
            process_input_item_actions.after(process_input_hotbar),
            process_input_hotbar,
            process_input_movement,
        )
            .run_if(in_state(GameState::Dimension)),
    );
}
