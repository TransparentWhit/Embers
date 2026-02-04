use super::{AttributeBase, Attributes, living_actor};
use crate::GameState;
use crate::dim::actor::living::attributes::embers;
use crate::dim::item::inv::{
    Inventory, InventorySlot, ItemDestination, ItemMoveQuantity, ItemSource, MoveItemCommandExt,
};
use crate::dim::item::{
    HandActionWield, ItemAction, ItemActionEnvironment, ItemActionWield, ItemActions, ItemStack,
};
use crate::dim::item::{InitialItemActions, ItemActionSlot};
use crate::dim::{
    ActionStatus, ActionStatusComponent, Actions, ActionsComponent, CollisionLayer,
    EntityInteraction, EntityInteractionEnvironment, EntityInteractions, Interactable,
    update_action,
};
use crate::input::{DoubleClicks, InputButton, InteractionTrigger, just_pressed, pressed};
use crate::reg::{OrRegistry, Reg, Registry};
use crate::ui::dim::PlayerCamera;
use crate::utils::{Keyed, NamespacedKey};
use avian3d::prelude::*;
use bevy::ecs::schedule::ScheduleConfigs;
use bevy::ecs::system::ScheduleSystem;
use bevy::input::mouse::{AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
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

static ENTITY_INTERACTION_COLLIDER: LazyLock<Collider> =
    LazyLock::new(|| Collider::cylinder(3., 2.));

fn process_input_entity_interactions_schedule() -> ScheduleConfigs<ScheduleSystem> {
    |spatial_query: SpatialQuery,
     keys: Res<ButtonInput<KeyCode>>,
     mouse: Res<ButtonInput<MouseButton>>,
     double_clicks: Res<DoubleClicks>,
     mut player: Single<(Entity, &mut PlayerEntityInteractions, &GlobalTransform), With<Player>>,
     interactables: Query<(&Interactable, &GlobalTransform)>,
     entity_interaction_reg: Reg<EntityInteraction>|
     -> (Entity, (), Option<InteractionTrigger>, Option<Entity>) {
        let (player, ref mut player_interactions, transform) = *player;
        **player_interactions = PlayerEntityInteractions::default();
        (
            player,
            (),
            if double_clicks.double_clicked(*CONTROLS_INTERACT.read().unwrap()) {
                Some(InteractionTrigger::DoubleClick)
            } else if pressed(&CONTROLS_INTERACT, &keys, &mouse) {
                Some(InteractionTrigger::Click)
            } else {
                None
            },
            spatial_query
                .shape_intersections(
                    &ENTITY_INTERACTION_COLLIDER,
                    transform.translation(),
                    transform.rotation(),
                    &SpatialQueryFilter::from_mask(CollisionLayer::Interactable),
                )
                .iter()
                .filter_map(|entity| {
                    interactables
                        .get(*entity)
                        .ok()
                        .map(|(interactable, interactable_transform)| {
                            (
                                interactable_transform
                                    .translation()
                                    .distance(transform.translation())
                                    * interactable.distance_factor,
                                *entity,
                            )
                        })
                })
                .min_by(|(lhs_distance, _lhs_entity), (rhs_distance, _rhs_entity)| {
                    f32::total_cmp(lhs_distance, rhs_distance)
                })
                .map(|(_distance, entity)| {
                    let (interactable, _interactable_transform) =
                        interactables.get(entity).unwrap();
                    for trigger in [InteractionTrigger::DoubleClick, InteractionTrigger::Click] {
                        interactable
                            .get_initial_interaction(trigger)
                            .as_ref()
                            .and_then(|interaction_key| entity_interaction_reg.get(interaction_key))
                            .map(|interaction| {
                                player_interactions
                                    .0
                                    .set(InteractionTrigger::Click, interaction.clone())
                            });
                    }
                    entity
                }),
        )
    }
    .pipe(
        update_action::<
            EntityInteraction,
            (),
            PlayerEntityInteractionStatus,
            PlayerEntityInteractions,
            With<Player>,
            EntityInteractionEnvironment<'static, 'static>,
        >,
    )
    .pipe(
        |mut interaction_status: Single<&mut PlayerEntityInteractionStatus, With<Player>>,
         time: Res<Time>| interaction_status.tick(time.delta()),
    )
    .into_configs()
}

fn process_input_item_actions_schedule() -> ScheduleConfigs<ScheduleSystem> {
    let update_slot_action_system =
        |equipment_slot: EquipmentSlot, control: &'static RwLock<InputButton>| {
            IntoSystem::into_system(
                move |keys: Res<ButtonInput<KeyCode>>,
                      mouse: Res<ButtonInput<MouseButton>>,
                      double_clicks: Res<DoubleClicks>,
                      player: Single<
                    (
                        Entity,
                        &PlayerItemActionStatus,
                        &PlayerEquipmentItemActions,
                        &PlayerInventory,
                        Ref<SelectedHotbarSlot>,
                    ),
                    With<Player>,
                >,
                      off_hand_swapped_reader: MessageReader<OffHandSwapped>|
                      -> (
                    Entity,
                    EquipmentSlot,
                    Option<InteractionTrigger>,
                    Option<Entity>,
                ) {
                    let (
                        player,
                        action_status,
                        equipment_actions,
                        inventory,
                        ref selected_hotbar_slot,
                    ) = *player;
                    let mut trigger = if double_clicks.double_clicked(*control.read().unwrap()) {
                        Some(InteractionTrigger::DoubleClick)
                    } else if pressed(control, &keys, &mouse) {
                        Some(InteractionTrigger::Click)
                    } else {
                        None
                    };
                    if matches!(equipment_slot, EquipmentSlot::OffHand)
                        && match *action_status.get(EquipmentSlot::MainHand) {
                            ActionStatus::Idle => false,
                            ActionStatus::Active {
                                trigger: current_main_hand_trigger,
                                ..
                            } => equipment_actions
                                .main_hand()
                                .get(current_main_hand_trigger)
                                .map(|main_hand_action| match main_hand_action.wield {
                                    ItemActionWield::Armor => unreachable!(),
                                    ItemActionWield::Hands(HandActionWield::Single) => trigger
                                        .and_then(|trigger| {
                                            equipment_actions.off_hand().get(trigger)
                                        })
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
                        || selected_hotbar_slot.is_changed()
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
                    (
                        player,
                        equipment_slot,
                        trigger,
                        inventory.equipment_slot(equipment_slot, selected_hotbar_slot.0),
                    )
                }
                .pipe(
                    update_action::<
                        ItemAction,
                        EquipmentSlot,
                        PlayerItemActionStatus,
                        PlayerEquipmentItemActions,
                        With<Player>,
                        ItemActionEnvironment<'static, 'static>,
                    >,
                ),
            )
        };
    (
        |mut player: Single<
            (
                &PlayerInventory,
                &SelectedHotbarSlot,
                &mut PlayerEquipmentItemActions,
            ),
            With<Player>,
        >,
         item_action_reg: Reg<ItemAction>,
         initial_item_actions: Query<(&ItemStack, Option<&InitialItemActions>)>,
         initial_item_actions_reg: Reg<InitialItemActions>| {
            let (inventory, selected_hotbar_slot, ref mut equipment_actions) = *player;
            update_equipment_slot_actions(
                EquipmentSlot::MainHand,
                inventory.main_hand(),
                &item_action_reg,
                initial_item_actions,
                &initial_item_actions_reg,
                equipment_actions,
            );
            update_equipment_slot_actions(
                EquipmentSlot::OffHand,
                inventory.hotbar(selected_hotbar_slot.0),
                &item_action_reg,
                initial_item_actions,
                &initial_item_actions_reg,
                equipment_actions,
            );
        },
        (
            update_slot_action_system(EquipmentSlot::MainHand, &CONTROLS_USE_MAIN_HAND),
            update_slot_action_system(EquipmentSlot::OffHand, &CONTROLS_USE_OFF_HAND),
            update_slot_action_system(EquipmentSlot::Armor, &CONTROLS_USE_ARMOR),
        ),
        |mut action_status: Single<&mut PlayerItemActionStatus, With<Player>>, time: Res<Time>| {
            action_status.tick(time.delta())
        },
    )
        .chain()
}

#[derive(Message)]
pub struct OffHandSwapped;

fn update_equipment_slot_actions(
    equipment_slot: EquipmentSlot,
    slot: Option<Entity>,
    item_action_reg: &Reg<ItemAction>,
    initial_item_actions: Query<(&ItemStack, Option<&InitialItemActions>)>,
    initial_item_actions_reg: &Reg<InitialItemActions>,
    equipment_actions: &mut PlayerEquipmentItemActions,
) {
    let slot_actions = equipment_actions.get_slot_mut(equipment_slot);
    *slot_actions = ItemActions::default();
    slot.and_then(|item| initial_item_actions.get(item).ok())
        .and_then(|(item_stack, initial_item_actions)| {
            initial_item_actions
                .cloned()
                .or_registry(&initial_item_actions_reg, item_stack.key())
        })
        .inspect(|initial_actions| {
            let item_action_slot = equipment_slot.item_action_slot();
            for trigger in [InteractionTrigger::DoubleClick, InteractionTrigger::Click] {
                initial_actions
                    .get(item_action_slot, trigger)
                    .and_then(|action| item_action_reg.get(action))
                    .inspect(|action| slot_actions.set(trigger, (*action).clone()));
            }
        });
}

pub fn process_input_hotbar(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
    player: Single<(Entity, &PlayerInventory, &SelectedHotbarSlot), With<Player>>,
    mut off_hand_swapped_writer: MessageWriter<OffHandSwapped>,
) {
    let (player, inventory, selected_hotbar_slot) = *player;
    let mut new_hotbar_selection = selected_hotbar_slot.0;
    for hotbar_slot in 0..HOTBAR_SLOTS {
        if just_pressed(&CONTROLS_HOTBARS[hotbar_slot as usize], &keys, &mouse) {
            new_hotbar_selection = hotbar_slot;
        }
    }
    if let MouseScrollUnit::Line = mouse_scroll.unit
        && let delta = mouse_scroll.delta.y as InventorySlot
        && delta != 0
    {
        new_hotbar_selection += delta;
        new_hotbar_selection = new_hotbar_selection.rem_euclid(HOTBAR_SLOTS);
    }
    if new_hotbar_selection != selected_hotbar_slot.0 {
        commands
            .entity(player)
            .insert(SelectedHotbarSlot(new_hotbar_selection));
    }
    if just_pressed(&CONTROLS_SWAP_OFF_HAND, &keys, &mouse) {
        commands.move_item(
            ItemSource::inventory_slot(player, PlayerInventory::MAIN_HAND_SLOT, inventory),
            ItemDestination::inventory_slot(player, new_hotbar_selection, inventory),
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
    PlayerEntityInteractionStatus,
    PlayerEntityInteractions,
    PlayerItemActionStatus,
    PlayerEquipmentItemActions,
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
    #[inline]
    pub fn armor(&self) -> Option<Entity> {
        self[Self::ARMOR_SLOT]
    }
    #[inline]
    pub fn main_hand(&self) -> Option<Entity> {
        self[Self::MAIN_HAND_SLOT]
    }
    #[inline]
    pub fn hotbar(&self, slot: InventorySlot) -> Option<Entity> {
        debug_assert!(
            Self::HOTBAR_SLOTS.contains(&slot),
            "Slot out of bounds for player hotbar: {}",
            slot
        );
        self[slot]
    }
    #[inline]
    pub fn equipment_slot(
        &self,
        equipment_slot: EquipmentSlot,
        selected_hotbar_slot: InventorySlot,
    ) -> Option<Entity> {
        match equipment_slot {
            EquipmentSlot::MainHand => self.main_hand(),
            EquipmentSlot::OffHand => self.hotbar(selected_hotbar_slot),
            EquipmentSlot::Armor => self.armor(),
        }
    }
}

#[derive(Component, Debug)]
pub struct SelectedHotbarSlot(pub InventorySlot);

impl Default for SelectedHotbarSlot {
    fn default() -> Self {
        Self(0)
    }
}

#[derive(Component, Debug, Default)]
pub struct PlayerEntityInteractionStatus(ActionStatus);

impl ActionStatusComponent for PlayerEntityInteractionStatus {
    type Key = ();
    fn get_action_status(&self, _key: &Self::Key) -> &ActionStatus {
        &self.0
    }
    fn get_action_status_mut(&mut self, _key: &Self::Key) -> &mut ActionStatus {
        &mut self.0
    }
}

impl PlayerEntityInteractionStatus {
    fn tick(&mut self, delta: Duration) {
        if let ActionStatus::Active { timer, .. } = &mut self.0 {
            timer.tick(delta);
        }
    }
}

#[derive(Component, Default)]
pub struct PlayerEntityInteractions(EntityInteractions);

impl ActionsComponent<EntityInteraction> for PlayerEntityInteractions {
    type Key = ();
    fn get_actions(&self, _key: &Self::Key) -> &Actions<EntityInteraction> {
        &self.0
    }
    fn get_actions_mut(&mut self, _key: &Self::Key) -> &mut Actions<EntityInteraction> {
        &mut self.0
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
pub struct PlayerItemActionStatus {
    main_hand: ActionStatus,
    off_hand: ActionStatus,
    armor: ActionStatus,
}

impl ActionStatusComponent for PlayerItemActionStatus {
    type Key = EquipmentSlot;
    #[inline]
    fn get_action_status(&self, key: &Self::Key) -> &ActionStatus {
        self.get(*key)
    }
    #[inline]
    fn get_action_status_mut(&mut self, key: &Self::Key) -> &mut ActionStatus {
        self.get_mut(*key)
    }
}

impl PlayerItemActionStatus {
    fn get(&self, slot: EquipmentSlot) -> &ActionStatus {
        match slot {
            EquipmentSlot::MainHand => &self.main_hand,
            EquipmentSlot::OffHand => &self.off_hand,
            EquipmentSlot::Armor => &self.armor,
        }
    }
    fn get_mut(&mut self, slot: EquipmentSlot) -> &mut ActionStatus {
        match slot {
            EquipmentSlot::MainHand => &mut self.main_hand,
            EquipmentSlot::OffHand => &mut self.off_hand,
            EquipmentSlot::Armor => &mut self.armor,
        }
    }
    fn tick(&mut self, delta: Duration) {
        for action_status in [&mut self.main_hand, &mut self.off_hand, &mut self.armor] {
            if let ActionStatus::Active { timer, .. } = action_status {
                timer.tick(delta);
            }
        }
    }
}

#[derive(Component, Default)]
pub struct PlayerEquipmentItemActions {
    main_hand: ItemActions,
    off_hand: ItemActions,
    armor: ItemActions,
}

impl ActionsComponent<ItemAction> for PlayerEquipmentItemActions {
    type Key = EquipmentSlot;
    #[inline]
    fn get_actions(&self, key: &Self::Key) -> &Actions<ItemAction> {
        self.get_slot(*key)
    }
    #[inline]
    fn get_actions_mut(&mut self, key: &Self::Key) -> &mut Actions<ItemAction> {
        self.get_slot_mut(*key)
    }
}

impl PlayerEquipmentItemActions {
    pub fn get_slot(&self, slot: EquipmentSlot) -> &ItemActions {
        match slot {
            EquipmentSlot::MainHand => &self.main_hand,
            EquipmentSlot::OffHand => &self.off_hand,
            EquipmentSlot::Armor => &self.armor,
        }
    }
    pub fn get_slot_mut(&mut self, slot: EquipmentSlot) -> &mut ItemActions {
        match slot {
            EquipmentSlot::MainHand => &mut self.main_hand,
            EquipmentSlot::OffHand => &mut self.off_hand,
            EquipmentSlot::Armor => &mut self.armor,
        }
    }
    pub fn main_hand(&self) -> &ItemActions {
        &self.main_hand
    }
    pub fn off_hand(&self) -> &ItemActions {
        &self.off_hand
    }
    pub fn armor(&self) -> &ItemActions {
        &self.armor
    }
    pub fn clear_slot(&mut self, slot: EquipmentSlot) {
        *self.get_slot_mut(slot) = ItemActions::default();
    }
}

pub fn player(attribute_bases: &Registry<AttributeBase>) -> impl Bundle {
    (
        living_actor(&KEY, attribute_bases, false),
        Collider::cylinder(0.5, 1.7),
        Player {
            flops: 0,
            hashes: 0,
            time_crystals: 0,
        },
    )
}

pub(in crate::dim) fn plugin(app: &mut App) {
    app.add_message::<OffHandSwapped>().add_systems(
        Update,
        (
            process_input_entity_interactions_schedule(),
            process_input_item_actions_schedule().after(process_input_hotbar),
            process_input_hotbar,
            process_input_movement,
        )
            .run_if(in_state(GameState::Dimension)),
    );
}
