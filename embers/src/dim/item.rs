pub mod inv;

use super::actor::living::player::Player;
use super::actor::primed_tnt::primed_tnt;
use super::{Action, ActionSlots, CollisionLayer, exclude_source};
use crate::input::InteractionTrigger;
use crate::pld::{
    Boxed, EMBERS_PAYLOAD_SOURCE_UUID, InjectedPayloads, PayloadManager, Payloads,
    inject_embers_payload_batch, inject_keyed_embers_payload_batch, resolve_payload,
};
use crate::utils::physics::section;
use crate::utils::{DynCmp, DynPartialCmp, Keyed, NamespacedKey};
use anyhow::Error;
use avian3d::prelude::*;
use bevy::ecs::system::{StaticSystemParam, SystemParam};
use bevy::ecs::template::TemplateContext;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;
use bevy::reflect::DynamicTypePath;
use derive_where::derive_where;
use embers_macros::identify;
use serde::{Deserialize, Serialize};
use std::iter::once;
use std::marker::PhantomData;
use std::sync::{Arc, LazyLock};
use std::time::Duration;
use thiserror::Error;
use toml::{Table, Value};
use uuid::Uuid;

pub mod embers {
    macro_rules! item {
        ($id: ident, $key: expr) => {
            pub static $id: std::sync::LazyLock<$crate::utils::NamespacedKey> =
                std::sync::LazyLock::new(|| $crate::utils::NamespacedKey::new_embers($key));
        };
    }
    item!(SPEAR, "spear");
    item!(SWORD, "sword");
    item!(TNT, "tnt");
}

#[derive(Component, Deserialize, Serialize, Clone, Debug, Eq, Hash, PartialEq)]
#[require(StackCount)]
#[serde(transparent)]
pub struct ItemStack(NamespacedKey);

static DEFAULT_ITEM_KEY: LazyLock<NamespacedKey> =
    LazyLock::new(|| NamespacedKey::new("_", "missingno"));

impl Default for ItemStack {
    fn default() -> Self {
        warn!("An default item stack is used! This is likely an error.");
        Self(DEFAULT_ITEM_KEY.clone())
    }
}

impl Keyed for ItemStack {
    fn key(&self) -> &NamespacedKey {
        &self.0
    }
}

impl ItemStack {
    pub fn new(key: NamespacedKey) -> Self {
        Self(key)
    }
}

struct ItemStackTemplate {
    key: NamespacedKey,
}

impl Default for ItemStackTemplate {
    fn default() -> Self {
        Self {
            key: DEFAULT_ITEM_KEY.clone(),
        }
    }
}

impl Template for ItemStackTemplate {
    type Output = ItemStack;
    fn build_template(&self, context: &mut TemplateContext) -> Result<Self::Output> {
        for component in context
            .resource::<Assets<BoxedItemComponent>>()
            .iter()
            .map(|(_id, component)| component.dyn_clone())
            .collect::<Box<[_]>>()
        {
            component.insert_prototype(context.entity, &self.key);
        }
        Ok(ItemStack(self.key.clone()))
    }
    fn clone_template(&self) -> Self {
        Self {
            key: self.key.clone(),
        }
    }
}

#[derive(Clone, Component, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct StackCount(u8);

impl Default for StackCount {
    fn default() -> Self {
        Self(1)
    }
}

pub trait ItemComponent:
    DynamicTypePath + Keyed + for<'world> DynCmp<EntityRef<'world>> + Send + Sync + 'static
{
    fn dyn_clone(&self) -> Box<dyn ItemComponent>;
    fn clear_prototypes(&self, world: &mut DeferredWorld);
    fn inject_prototype(&self, world: &mut DeferredWorld, item: &NamespacedKey, prototype: Value);
    fn insert_prototype(&self, item_stack: &mut EntityWorldMut, item: &NamespacedKey);
}

#[derive(TypePath)]
#[doc(hidden)]
pub enum DynItemComponent {}

pub type BoxedItemComponent = Boxed<DynItemComponent, dyn ItemComponent>;

#[derive_where(Clone)]
#[derive(TypePath)]
pub struct StandardItemComponent<C: Clone + Component + for<'de> Deserialize<'de> + Eq + TypePath> {
    source_uuid: Uuid,
    key: NamespacedKey,
    _marker: PhantomData<fn() -> C>,
}

impl<C: Clone + Component + for<'de> Deserialize<'de> + Eq + TypePath> Keyed
    for StandardItemComponent<C>
{
    fn key(&self) -> &NamespacedKey {
        &self.key
    }
}

impl<C: Clone + Component + for<'de> Deserialize<'de> + Eq + TypePath>
    DynPartialCmp<EntityRef<'_>, EntityRef<'_>> for StandardItemComponent<C>
{
    fn dyn_eq(&self, lhs: EntityRef<'_>, rhs: EntityRef<'_>) -> bool {
        match (lhs.get::<C>(), rhs.get::<C>()) {
            (Some(lhs), Some(rhs)) => *lhs == *rhs,
            (None, None) => true,
            _ => false,
        }
    }
}

impl<C: Clone + Component + for<'de> Deserialize<'de> + Eq + TypePath> DynCmp<EntityRef<'_>>
    for StandardItemComponent<C>
{
}

impl<C: Clone + Component + for<'de> Deserialize<'de> + Eq + TypePath> StandardItemComponent<C> {
    pub fn new(source_uuid: Uuid, key: NamespacedKey) -> BoxedItemComponent {
        BoxedItemComponent::new_boxed(Box::new(Self {
            source_uuid,
            key,
            _marker: PhantomData,
        }))
    }
    #[inline]
    pub fn new_embers(key: NamespacedKey) -> BoxedItemComponent {
        Self::new(EMBERS_PAYLOAD_SOURCE_UUID.clone(), key)
    }
}

impl<C: Clone + Component + for<'de> Deserialize<'de> + Eq + TypePath> ItemComponent
    for StandardItemComponent<C>
{
    fn dyn_clone(&self) -> Box<dyn ItemComponent> {
        Box::new(self.clone())
    }
    fn clear_prototypes(&self, world: &mut DeferredWorld) {
        world
            .resource_mut::<Assets<ItemComponentPrototype<C>>>()
            .clear();
    }
    fn inject_prototype(&self, world: &mut DeferredWorld, item: &NamespacedKey, value: Value) {
        let resource_entities = world.resource_entities();
        let [mut prototypes, mut injected_payloads] = world.entity_mut([
            resource_entities
                .get(
                    world
                        .component_id::<Assets<ItemComponentPrototype<C>>>()
                        .unwrap(),
                )
                .unwrap(),
            resource_entities
                .get(world.component_id::<InjectedPayloads>().unwrap())
                .unwrap(),
        ]);
        prototypes
            .get_mut::<Assets<ItemComponentPrototype<C>>>()
            .unwrap()
            .inject(
                &mut injected_payloads.get_mut::<InjectedPayloads>().unwrap(),
                self.source_uuid,
                format!("item_component_prototypes/{}", item.path_string()),
                ItemComponentPrototype(C::deserialize(value).unwrap()),
            );
    }
    fn insert_prototype(&self, item_stack: &mut EntityWorldMut, item: &NamespacedKey) {
        if let Some(ItemComponentPrototype(prototype)) = resolve_payload(
            item_stack.resource::<PayloadManager>(),
            item_stack.resource::<AssetServer>(),
            item_stack.resource::<Assets<ItemComponentPrototype<C>>>(),
            item,
        ) {
            item_stack.insert(prototype.clone());
        }
    }
}

#[derive(Asset, TypePath)]
pub struct ItemComponentPrototype<C: Component + TypePath>(pub C);

#[derive(Clone, Component, Debug, Deserialize, Eq, Hash, PartialEq, Serialize, TypePath)]
#[require(ItemStack)]
pub struct Enchantments();

impl Default for Enchantments {
    fn default() -> Self {
        Self()
    }
}

#[derive(Clone, Component, Debug, Deserialize, Eq, Hash, PartialEq, Serialize, TypePath)]
#[require(ItemStack)]
pub struct InitialItemActions {
    hands_click: Option<NamespacedKey>,
    hands_double_click: Option<NamespacedKey>,
    armor_click: Option<NamespacedKey>,
    armor_double_click: Option<NamespacedKey>,
}

impl InitialItemActions {
    pub fn get(&self, slot: ItemActionSlot, trigger: InteractionTrigger) -> Option<&NamespacedKey> {
        match (slot, trigger) {
            (ItemActionSlot::Hands, InteractionTrigger::Click) => self.hands_click.as_ref(),
            (ItemActionSlot::Hands, InteractionTrigger::DoubleClick) => {
                self.hands_double_click.as_ref()
            }
            (ItemActionSlot::Armor, InteractionTrigger::Click) => self.armor_click.as_ref(),
            (ItemActionSlot::Armor, InteractionTrigger::DoubleClick) => {
                self.armor_double_click.as_ref()
            }
        }
    }
}

#[derive(Clone, Component, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize, TypePath)]
#[require(ItemStack)]
#[serde(transparent)]
pub struct MaxStackSize(u8);

impl Default for MaxStackSize {
    fn default() -> Self {
        Self(1)
    }
}

#[derive(Clone, Component, Debug, Deserialize, Eq, Hash, PartialEq, Serialize, TypePath)]
#[require(ItemStack)]
pub struct RangedAmmo();

#[derive(Clone, Component, Debug, Deserialize, Serialize, TypePath)]
#[require(ItemStack)]
#[serde(transparent)]
pub struct Weight(f32);

impl PartialEq for Weight {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 || (self.0.is_nan() && other.0.is_nan())
    }
}

impl Eq for Weight {}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ItemActionSlot {
    Armor,
    Hands,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ItemActionWield {
    Armor,
    Hands(HandActionWield),
}

impl Default for ItemActionWield {
    fn default() -> Self {
        Self::Hands(default())
    }
}

impl ItemActionWield {
    pub fn slot(&self) -> ItemActionSlot {
        match self {
            Self::Armor => ItemActionSlot::Armor,
            Self::Hands(..) => ItemActionSlot::Hands,
        }
    }
}

#[derive(Deserialize, Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum HandActionWield {
    #[default]
    Single,
    Dual,
}

pub trait ItemActionBuilder: Send + Sync {
    fn build(&self, key: NamespacedKey, config: Table) -> Result<ItemAction, Error>;
}

impl<T: (Fn(NamespacedKey, Table) -> Result<ItemAction, Error>) + Send + Sync> ItemActionBuilder
    for T
{
    fn build(&self, key: NamespacedKey, config: Table) -> Result<ItemAction, Error> {
        self(key, config)
    }
}

#[derive(TypePath)]
#[doc(hidden)]
pub enum DynItemActionBuilder {}

pub type BoxedItemActionBuilder = Boxed<DynItemActionBuilder, dyn ItemActionBuilder>;

#[derive(SystemParam)]
pub struct ItemActionEnvironment<'w, 's> {
    commands: Commands<'w, 's>,
    spatial_query: SpatialQuery<'w, 's>,
    asset_server: Res<'w, AssetServer>,
    player: Single<'w, 's, (Entity, &'static Transform), With<Player>>,
}

pub type ItemActionSlots = ActionSlots<ItemAction>;

#[derive(Asset, Clone, TypePath)]
#[identify(key)]
// TODO inspect do we need to clone this?
pub struct ItemAction {
    key: NamespacedKey,
    on_begin: Arc<dyn Fn(&mut ItemActionEnvironment, Entity) + Send + Sync>,
    on_end: Arc<
        dyn Fn(&mut ItemActionEnvironment, Entity, Option<Duration>) -> Option<NamespacedKey>
            + Send
            + Sync,
    >,
    pub wield: ItemActionWield,
    duration: Duration,
}

impl ItemAction {
    pub fn new(
        key: NamespacedKey,
        on_begin: impl Fn(&mut ItemActionEnvironment, Entity) + Send + Sync + 'static,
        on_end: impl Fn(&mut ItemActionEnvironment, Entity, Option<Duration>) -> Option<NamespacedKey>
        + Send
        + Sync
        + 'static,
        wield: ItemActionWield,
        duration: Duration,
    ) -> Self {
        Self {
            key,
            on_begin: Arc::new(on_begin),
            on_end: Arc::new(on_end),
            wield,
            duration,
        }
    }
}

impl Keyed for ItemAction {
    fn key(&self) -> &NamespacedKey {
        &self.key
    }
}

impl Action for ItemAction {
    type Environment = ItemActionEnvironment<'static, 'static>;
    fn on_begin(&self, environment: &mut StaticSystemParam<Self::Environment>, item: Entity) {
        (self.on_begin)(environment, item)
    }
    fn on_end(
        &self,
        environment: &mut StaticSystemParam<Self::Environment>,
        item: Entity,
        duration: Option<Duration>,
    ) -> Option<NamespacedKey> {
        (self.on_end)(environment, item, duration)
    }
    fn duration(&self) -> Duration {
        self.duration
    }
}

pub fn item_stack(key: NamespacedKey) -> impl Scene {
    template_value(ItemStackTemplate { key })
}

pub(super) fn plugin(app: &mut App) {
    app
        .init_asset::<ItemAction>()
        .init_asset::<BoxedItemActionBuilder>()
        .add_systems(PreStartup, inject_embers_payload_batch::<BoxedItemActionBuilder>("item_action_builders/{}", [
            (NamespacedKey::new_embers("melee"), Box::new(|key, config| {
                #[derive(Deserialize)]
                struct Melee {
                    damage: f32,
                    arc_deg: f32,
                    range: f32,
                    wield: HandActionWield,
                    duration_secs: f32,
                    next_action: Option<String>,
                }
                let action = Melee::deserialize(config)?;
                #[derive(Debug, Error)]
                #[error("Couldn't create a collider for the given arc_deg({arc_deg}) and range({range}).")]
                struct NoCollider {
                    arc_deg: f32,
                    range: f32,
                }
                let collider = section(action.arc_deg.to_radians(), action.range, 1.).ok_or(NoCollider {
                    arc_deg: action.arc_deg,
                    range: action.range,
                })?;
                let spatial_query_filter = SpatialQueryFilter::from_mask(CollisionLayer::LivingActor);
                let next_action = action.next_action.as_ref().and_then(|next| {
                    NamespacedKey::try_from_with_namespaced(next.as_str(), &key)
                        .ok()
                });
                Ok(ItemAction::new(
                    key,
                    |_environment, _item| {},
                    move |ItemActionEnvironment {
                            commands,
                            spatial_query,
                            asset_server: _,
                            player,
                        }, item, duration| {
                            let (player, transform) = **player;
                            if duration.is_none() {
                                for entity in spatial_query.shape_intersections(&collider, transform.translation, transform.rotation, &spatial_query_filter.clone().with_excluded_entities(once(player))) {
                                    // todo
                                }
                                next_action.clone()
                            } else {
                                None
                            }
                        },
                    ItemActionWield::Hands(action.wield),
                    Duration::from_secs_f32(action.duration_secs),
                ))
            }) as Box<dyn ItemActionBuilder>),
            (NamespacedKey::new_embers("throw"), Box::new(|key, config| {
                #[derive(Deserialize)]
                struct Throw {
                    velocity: f32,
                    wield: HandActionWield,
                    timeout_secs: f32,
                    next_action: Option<String>,
                }
                let action = Throw::deserialize(config)?;
                let velocity = action.velocity;
                let next_action = action.next_action.as_ref().and_then(|next| {
                    NamespacedKey::try_from_with_namespaced(next.as_str(), &key)
                        .ok()
                });
                Ok(ItemAction::new(
                    key,
                move |ItemActionEnvironment {
                        commands,
                        spatial_query: _,
                        asset_server,
                        player,
                    }, _item| {
                        let (player, transform) = **player;
                        commands.spawn_scene(bsn! {
                            primed_tnt()
                            // exclude_source(player) TODO
                            template_value({transform.clone()})
                            LinearVelocity({transform.rotation * -Vec3::Z * velocity})
                        });
                    },
                    move |_environment, _item, _duration| next_action.clone(),
                    ItemActionWield::Hands(action.wield),
                    Duration::from_secs_f32(action.timeout_secs),
                ))
            })),
            (NamespacedKey::new_embers("charged_throw"), Box::new(|key, config| {
                #[derive(Deserialize)]
                struct ChargedThrow {
                    wield: HandActionWield,
                    hold_threshold_secs: Option<f32>,
                    hold_action: Option<String>,
                }
                let action = ChargedThrow::deserialize(config)?;
                let hold_action = action.hold_action.as_ref().and_then(|next| {
                    NamespacedKey::try_from_with_namespaced(next.as_str(), &key)
                        .ok()
                });
                Ok(ItemAction::new(
                    key,
                    |_environment, _item| {},
                    move |ItemActionEnvironment {
                            commands,
                            spatial_query: _,
                            asset_server,
                            player,
                        }, _item, duration| {
                            if duration.is_none() {
                                hold_action.clone()
                            } else {
                                println!("throwing");
                                None
                            }
                        },
                    ItemActionWield::Hands(action.wield),
                    action.hold_threshold_secs.map_or(Duration::MAX, Duration::from_secs_f32),
                ))
            })),
        ]))
        .init_asset::<BoxedItemComponent>()
        .add_systems(PreStartup, inject_keyed_embers_payload_batch::<BoxedItemComponent>("item_components/{}", [
            StandardItemComponent::<Enchantments>::new_embers(NamespacedKey::new_embers("enchantments")),
            StandardItemComponent::<InitialItemActions>::new_embers(NamespacedKey::new_embers("initial_actions")),
            StandardItemComponent::<MaxStackSize>::new_embers(NamespacedKey::new_embers("max_stack_size")),
            StandardItemComponent::<RangedAmmo>::new_embers(NamespacedKey::new_embers("ranged_ammo")),
            StandardItemComponent::<Weight>::new_embers(NamespacedKey::new_embers("weight")),
        ]))
        .init_asset::<ItemComponentPrototype<Enchantments>>()
        .init_asset::<ItemComponentPrototype<InitialItemActions>>()
        .init_asset::<ItemComponentPrototype<MaxStackSize>>()
        .init_asset::<ItemComponentPrototype<RangedAmmo>>()
        .init_asset::<ItemComponentPrototype<Weight>>();
}
