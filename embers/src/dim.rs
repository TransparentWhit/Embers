pub mod actor;
pub mod block;
mod chunk;
pub mod item;

use crate::input::InteractionTrigger;
use crate::reg::{Reg, RegMut, RegistryError, RegistryInitExt};
use crate::ui::ActiveOverlay;
use crate::utils::{Keyed, NamespacedKey};
use actor::living::player;
use actor::living::player::{Player, PlayerInventory};
use avian3d::prelude::*;
use avian3d::schedule::LastPhysicsTick;
use bevy::ecs::change_detection::Tick;
use bevy::ecs::component::Mutable;
use bevy::ecs::query::QueryFilter;
use bevy::ecs::system::{StaticSystemParam, SystemParam};
use bevy::prelude::*;
use bevy::time::Stopwatch;
use bevy_tnua::builtins::{TnuaBuiltinCrouch, TnuaBuiltinDash};
use bevy_tnua::prelude::*;
use derive_where::derive_where;
use embers_macros::identify;
use item::inv::{ItemDestination, ItemMoveQuantity, ItemSource, MoveItemCommandExt};
use serde::{Deserialize, Serialize};
use std::ops::Neg;
use std::sync::{Arc, LazyLock};
use std::time::Duration;

pub mod embers {
    macro_rules! dim {
        ($id: ident, $key: expr) => {
            pub static $id: std::sync::LazyLock<$crate::utils::NamespacedKey> =
                std::sync::LazyLock::new(|| $crate::utils::NamespacedKey::new_embers($key));
        };
    }
    dim!(ASSEMBLY_APEX, "assembly_apex");
    dim!(LOBBY, "lobby");
}

#[derive(Component)]
#[require(Dimension)]
pub struct ActiveDimension;

#[derive(Component, Debug)]
pub struct Dimension(NamespacedKey);

static DEFAULT_DIMENSION_KEY: LazyLock<NamespacedKey> =
    LazyLock::new(|| NamespacedKey::new("_", "missingno"));

impl Default for Dimension {
    fn default() -> Self {
        warn!("An default dimension is used! This is likely an error.");
        Self::new(DEFAULT_DIMENSION_KEY.clone())
    }
}

impl Keyed for Dimension {
    fn key(&self) -> &NamespacedKey {
        &self.0
    }
}

impl Dimension {
    pub fn new(key: NamespacedKey) -> Self {
        Self(key)
    }
}

#[derive(Debug, Event)]
pub struct DimensionGenerationRequest(NamespacedKey);

impl DimensionGenerationRequest {
    pub fn new(key: &impl Keyed) -> Self {
        Self(key.key().clone())
    }
}

fn handle_dimension_generation_request(
    request: On<DimensionGenerationRequest>,
    mut commands: Commands,
) {
    let DimensionGenerationRequest(key) = &*request;
    commands.spawn(Dimension::new(key.clone()));
}

#[derive(Deserialize, Serialize, Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum Direction {
    None,
    East,
    West,
    Up,
    Down,
    South,
    North,
}

impl Direction {
    #[inline]
    pub const fn is_cartesian(&self) -> bool {
        matches!(
            self,
            Self::East | Self::West | Self::Up | Self::Down | Self::South | Self::North
        )
    }
}

impl Neg for Direction {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self::Output {
        match self {
            Self::None => Self::None,
            Self::East => Self::West,
            Self::West => Self::East,
            Self::Up => Self::Down,
            Self::Down => Self::Up,
            Self::South => Self::North,
            Self::North => Self::South,
        }
    }
}

impl From<Direction> for Vec3 {
    #[inline]
    fn from(value: Direction) -> Self {
        match value {
            Direction::None => Self::ZERO,
            Direction::East => Self::X,
            Direction::West => Self::NEG_X,
            Direction::Up => Self::Y,
            Direction::Down => Self::NEG_Y,
            Direction::North => Self::Z,
            Direction::South => Self::NEG_Z,
        }
    }
}

impl From<Direction> for IVec3 {
    #[inline]
    fn from(value: Direction) -> Self {
        match value {
            Direction::None => Self::ZERO,
            Direction::East => Self::X,
            Direction::West => Self::NEG_X,
            Direction::Up => Self::Y,
            Direction::Down => Self::NEG_Y,
            Direction::North => Self::Z,
            Direction::South => Self::NEG_Z,
        }
    }
}

type Physics = (CollisionLayers, Dominance, LockedAxes, RigidBody);

const FREE: LockedAxes = LockedAxes::new();
const LOCK_XZ_ROTATION: LockedAxes = LockedAxes::new().lock_rotation_x().lock_rotation_z();

#[derive(PhysicsLayer, Default, Copy, Clone)]
enum CollisionLayer {
    Interactable,
    LivingActor,
    MiscActor,
    #[default]
    Phantom,
    Projectile,
    Environment,
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum PhysicsPreset {
    LivingActor,
    MiscActor,
    Phantom,
    Projectile,
    Environment,
}

impl PhysicsPreset {
    #[inline]
    pub fn physics(&self, interactable: bool) -> Physics {
        (
            CollisionLayers::new(
                LayerMask(
                    match self {
                        Self::LivingActor => CollisionLayer::LivingActor,
                        Self::MiscActor => CollisionLayer::MiscActor,
                        Self::Phantom => CollisionLayer::Phantom,
                        Self::Projectile => CollisionLayer::Projectile,
                        Self::Environment => CollisionLayer::Environment,
                    }
                    .to_bits()
                        | if interactable {
                            CollisionLayer::Interactable.to_bits()
                        } else {
                            0
                        },
                ),
                match self {
                    Self::Phantom => [
                        CollisionLayer::LivingActor,
                        CollisionLayer::MiscActor,
                        CollisionLayer::Projectile,
                        CollisionLayer::Environment,
                    ]
                    .into(),
                    Self::Environment => [
                        CollisionLayer::LivingActor,
                        CollisionLayer::MiscActor,
                        CollisionLayer::Phantom,
                        CollisionLayer::Projectile,
                    ]
                    .into(),
                    _ => LayerMask::ALL,
                },
            ),
            Dominance(match self {
                Self::LivingActor => 3,
                Self::MiscActor => 2,
                Self::Phantom => 0,
                Self::Projectile => 1,
                Self::Environment => 4,
            }),
            match self {
                Self::LivingActor => LOCK_XZ_ROTATION,
                _ => FREE,
            },
            match self {
                Self::Environment => RigidBody::Static,
                _ => RigidBody::Dynamic,
            },
        )
    }
}

pub fn exclude_source(source: Entity) -> impl Bundle {
    (
        SourceExclusion(source, Tick::MAX),
        ActiveCollisionHooks::MODIFY_CONTACTS,
    )
}

#[derive(Component)]
#[component(storage = "SparseSet")] // TODO: Benchmark whether sparse set storage is actually more performant than table storage
struct SourceExclusion(Entity, Tick);

// TODO: Consider removing unused active collision hooks?

#[derive(SystemParam)]
pub(super) struct SourceExclusionCollisionHooks<'w, 's> {
    exclusions: Query<'w, 's, &'static SourceExclusion>,
    last_physics_tick: Res<'w, LastPhysicsTick>,
}

impl CollisionHooks for SourceExclusionCollisionHooks<'_, '_> {
    fn modify_contacts(&self, contacts: &mut ContactPair, commands: &mut Commands) -> bool {
        let mut exclude = |entity0, entity1| {
            if let Ok(exclusion) = self.exclusions.get(entity0) {
                if exclusion.1 == Tick::MAX
                    || self
                        .last_physics_tick
                        .0
                        .get()
                        .wrapping_sub(exclusion.1.get())
                        == 1
                {
                    if exclusion.0 == entity1 {
                        commands
                            .entity(entity0)
                            .insert(SourceExclusion(entity1, self.last_physics_tick.0));
                        return false;
                    }
                } else {
                    #[cfg(debug_assertions)]
                    if self.last_physics_tick.0.get() <= exclusion.1.get() {
                        warn!(
                            "Expected last exclusion tick ({}) to be smaller than current physics tick ({}).",
                            exclusion.1.get(),
                            self.last_physics_tick.0.get()
                        );
                    }
                    commands.entity(entity0).remove::<SourceExclusion>();
                }
            }
            true
        };
        exclude(contacts.collider1, contacts.collider2)
            && exclude(contacts.collider2, contacts.collider1)
    }
}

#[derive(TnuaScheme, Debug)]
#[scheme(basis = TnuaBuiltinWalk)]
pub enum Movements {
    Sneak(TnuaBuiltinCrouch),
    Roll(TnuaBuiltinDash),
}

pub trait Action: Keyed + Clone {
    type Environment: SystemParam;
    fn on_begin<'world, 'state>(
        &self,
        environment: &mut StaticSystemParam<'world, 'state, Self::Environment>,
        object: Entity,
    );
    fn on_end<'world, 'state>(
        &self,
        environment: &mut StaticSystemParam<'world, 'state, Self::Environment>,
        object: Entity,
        duration: Option<Duration>,
    ) -> Option<NamespacedKey>;
    fn duration(&self) -> Duration;
}

#[derive(Eq, PartialEq, Clone)]
#[derive_where(Default)]
pub struct Actions<A: Action> {
    click: Option<A>,
    double_click: Option<A>,
}

impl<A: Action> Actions<A> {
    pub fn get(&self, trigger: InteractionTrigger) -> Option<&A> {
        match trigger {
            InteractionTrigger::Click => self.click.as_ref(),
            InteractionTrigger::DoubleClick => self.double_click.as_ref(),
        }
    }
    pub fn get_mut(&mut self, trigger: InteractionTrigger) -> Option<&mut A> {
        match trigger {
            InteractionTrigger::Click => self.click.as_mut(),
            InteractionTrigger::DoubleClick => self.double_click.as_mut(),
        }
    }
    pub fn set(&mut self, trigger: InteractionTrigger, action: A) {
        match trigger {
            InteractionTrigger::Click => self.click = Some(action),
            InteractionTrigger::DoubleClick => self.double_click = Some(action),
        }
    }
    pub fn clear(&mut self, trigger: InteractionTrigger) {
        match trigger {
            InteractionTrigger::Click => self.click = None,
            InteractionTrigger::DoubleClick => self.double_click = None,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub enum ActionStatus {
    #[default]
    Idle,
    Active {
        timer: Stopwatch,
        trigger: InteractionTrigger,
    },
}

impl ActionStatus {
    pub fn idle() -> Self {
        Self::Idle
    }
    pub fn activate(trigger: InteractionTrigger) -> Self {
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

pub trait ActionStatusComponent: Component<Mutability = Mutable> {
    type Key;
    fn get_action_status(&self, key: &Self::Key) -> &ActionStatus;
    fn get_action_status_mut(&mut self, key: &Self::Key) -> &mut ActionStatus;
}

pub trait ActionsComponent<A: Action>: Component<Mutability = Mutable> {
    type Key;
    fn get_actions(&self, key: &Self::Key) -> &Actions<A>;
    fn get_actions_mut(&mut self, key: &Self::Key) -> &mut Actions<A>;
}

#[derive(Message)]
pub struct ActionInterruptionEvent {
    pub agent_entity: Entity,
    pub interruption: NamespacedKey,
}

fn update_action<
    A: Action<Environment = Env> + Send + Sync + 'static,
    Key: 'static,
    StatusComponent: ActionStatusComponent<Key = Key>,
    Component: ActionsComponent<A, Key = Key>,
    Filter: QueryFilter,
    Env: SystemParam + 'static,
>(
    (In(agent_entity), In(actions_key), In(mut trigger), In(object)): (
        In<Entity>,
        In<Key>,
        In<Option<InteractionTrigger>>,
        In<Option<Entity>>,
    ),
    mut agent: Query<(&mut StatusComponent, &mut Component), Filter>,
    mut environment: StaticSystemParam<Env>,
    action_reg: Reg<A>,
    mut interruption_events: MessageReader<ActionInterruptionEvent>,
) {
    let (ref mut status, ref mut actions) = agent.get_mut(agent_entity).unwrap();
    let status = status.get_action_status_mut(&actions_key);
    let actions = actions.get_actions_mut(&actions_key);
    //let environment = environment.into_inner();
    trigger.take_if(|active_trigger| {
        let interrupted = interruption_events.read().any(|event| {
            event.agent_entity == agent_entity
                && action_reg.is_tagged(
                    &event.interruption,
                    actions
                        .get(*active_trigger)
                        .expect("Should not be performing nonexistent action")
                        .key(),
                )
        });
        interruption_events.clear();
        interrupted
    });
    match trigger {
        Some(active_trigger) => {
            if status.is_idle() {
                if let Some(action) = actions.get(active_trigger) {
                    *status = ActionStatus::activate(active_trigger);
                    action.on_begin(
                        &mut environment,
                        object.expect("Action should not be performed on a nonexistent object."),
                    );
                }
            } else if let ActionStatus::Active {
                ref timer,
                trigger: current_trigger,
            } = *status
            {
                if let Some(action) = actions.get_mut(active_trigger) {
                    let finished = timer.elapsed() >= action.duration();
                    if finished || active_trigger != current_trigger {
                        if let Some(new_action) = action
                            .on_end(
                                &mut environment,
                                object.expect(
                                    "Action should not be performed on a nonexistent object.",
                                ),
                                if finished {
                                    None
                                } else {
                                    Some(timer.elapsed())
                                },
                            )
                            .and_then(|new_action| action_reg.get(&new_action).cloned())
                        {
                            *action = new_action;
                        }
                        *status = ActionStatus::activate(active_trigger);
                        action.on_begin(
                            &mut environment,
                            object
                                .expect("Action should not be performed on a nonexistent object."),
                        );
                    }
                } else if actions.get(current_trigger).is_none() {
                    *status = ActionStatus::idle();
                }
            }
        }
        None => {
            if let ActionStatus::Active { ref timer, trigger } = *status {
                if let Some(action) = actions.get_mut(trigger) {
                    if let Some(new_action) = action
                        .on_end(
                            &mut environment,
                            object
                                .expect("Action should not be performed on a nonexistent object."),
                            Some(timer.elapsed()).take_if(|used| action.duration() >= *used),
                        )
                        .and_then(|new_action| action_reg.get(&new_action).cloned())
                    {
                        *action = new_action;
                    }
                }
                *status = ActionStatus::idle();
            }
        }
    }
}

#[derive(SystemParam)]
pub struct EntityInteractionEnvironment<'w, 's> {
    commands: Commands<'w, 's>,
    player: Single<'w, 's, (Entity, &'static PlayerInventory, &'static Transform), With<Player>>,
}

pub type EntityInteractions = Actions<EntityInteraction>;

#[derive(Clone)]
#[identify(key)]
pub struct EntityInteraction {
    key: NamespacedKey,
    on_begin: Arc<dyn Fn(&mut EntityInteractionEnvironment, Entity) + Send + Sync>,
    on_end: Arc<
        dyn Fn(&mut EntityInteractionEnvironment, Entity, Option<Duration>) -> Option<NamespacedKey>
            + Send
            + Sync,
    >,
    duration: Duration,
}

impl EntityInteraction {
    pub fn new(
        key: NamespacedKey,
        on_begin: impl Fn(&mut EntityInteractionEnvironment, Entity) + Send + Sync + 'static,
        on_end: impl Fn(
            &mut EntityInteractionEnvironment,
            Entity,
            Option<Duration>,
        ) -> Option<NamespacedKey>
        + Send
        + Sync
        + 'static,
        duration: Duration,
    ) -> Self {
        Self {
            key,
            on_begin: Arc::new(on_begin),
            on_end: Arc::new(on_end),
            duration,
        }
    }
}

impl Keyed for EntityInteraction {
    fn key(&self) -> &NamespacedKey {
        &self.key
    }
}

impl Action for EntityInteraction {
    type Environment = EntityInteractionEnvironment<'static, 'static>;
    fn on_begin(&self, environment: &mut StaticSystemParam<Self::Environment>, entity: Entity) {
        (self.on_begin)(environment, entity)
    }
    fn on_end(
        &self,
        environment: &mut StaticSystemParam<Self::Environment>,
        entity: Entity,
        duration: Option<Duration>,
    ) -> Option<NamespacedKey> {
        (self.on_end)(environment, entity, duration)
    }
    fn duration(&self) -> Duration {
        self.duration
    }
}

#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct Interactable {
    /// The larger this is, the closer you need to get to interact
    pub distance_factor: f32,
    pub initial_click: Option<NamespacedKey>,
    pub initial_double_click: Option<NamespacedKey>,
}

impl Interactable {
    pub fn get_initial_interaction(&self, trigger: InteractionTrigger) -> Option<&NamespacedKey> {
        match trigger {
            InteractionTrigger::Click => self.initial_click.as_ref(),
            InteractionTrigger::DoubleClick => self.initial_double_click.as_ref(),
        }
    }
}

/// Time of the day, within [0, 1).
#[derive(Component)]
pub struct Time(pub f32);

impl Default for Time {
    fn default() -> Self {
        Self(0.25)
    }
}

#[derive(Component)]
struct DimensionalGateway;

pub static INTERACTION_LEVEL_SELECTION: LazyLock<NamespacedKey> =
    LazyLock::new(|| NamespacedKey::new_embers("level_selection"));

pub fn dimensional_gateway(asset_server: &AssetServer) -> impl Bundle {
    (
        DimensionalGateway,
        PhysicsPreset::Phantom.physics(true),
        Visibility::Visible,
        Mesh3d(asset_server.add(Cuboid::new(3., 1., 3.).mesh().build())),
        MeshMaterial3d(asset_server.add(StandardMaterial {
            base_color: Color::BLACK,
            ..default()
        })),
        Interactable {
            distance_factor: 1.,
            initial_click: Some(INTERACTION_LEVEL_SELECTION.clone()),
            initial_double_click: None,
        },
    )
}

pub(super) fn plugin(app: &mut App) {
    app.add_message::<ActionInterruptionEvent>()
        .init_registry::<EntityInteraction>()
        .add_systems(
            PreStartup,
            |mut entity_interactions: RegMut<EntityInteraction>| {
                (|| {
                    entity_interactions.register_keyed(EntityInteraction::new(
                        NamespacedKey::new_embers("item_actor/pickup"),
                        |_environment, _entity| {},
                        |EntityInteractionEnvironment { commands, player }, entity, _duration| {
                            let (player, inventory, _global_transform) = **player;
                            commands.move_item(
                                ItemSource::item_actor(entity),
                                ItemDestination::inventory_range(
                                    player,
                                    0..inventory.size(),
                                    inventory,
                                ),
                                ItemMoveQuantity::All,
                            );
                            None
                        },
                        Duration::from_millis(200),
                    ))?;
                    entity_interactions.register_keyed(EntityInteraction::new(
                        NamespacedKey::new_embers("level_selection"),
                        |EntityInteractionEnvironment {
                             commands,
                             player: _player,
                         },
                         _entity| {
                            commands.queue(|world: &mut World| {
                                world
                                    .resource_mut::<NextState<ActiveOverlay>>()
                                    .set(ActiveOverlay::GatewayMenu);
                            });
                        },
                        |_environment, _entity, _duration| None,
                        Duration::from_millis(200),
                    ))?;
                    Ok::<(), RegistryError>(())
                })()
                .expect("Failed to register entity interactions")
            },
        )
        .add_observer(handle_dimension_generation_request)
        .add_plugins(actor::plugin)
        .add_plugins(block::plugin)
        .add_plugins(item::plugin)
        .add_plugins(player::plugin);
}
