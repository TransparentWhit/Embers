pub mod inv;

use crate::dim::actor::living::player::Player;
use crate::dim::actor::primed_tnt::primed_tnt;
use crate::dim::{Action, Actions, CollisionLayer, exclude_source};
use crate::input::InteractionTrigger;
use crate::reg::{DynRegMut, DynamicRegistry, Registry, RegistryError, RegistryInitExt};
use crate::utils::physics::section;
use crate::utils::{Keyed, NamespacedKey, UntypedCmp, UntypedPartialCmp};
use anyhow::Error;
use avian3d::prelude::*;
use bevy::ecs::system::{StaticSystemParam, SystemParam};
use bevy::prelude::*;
use embers_macros::identify;
use serde::{Deserialize, Serialize};
use std::iter::once;
use std::marker::PhantomData;
use std::sync::{Arc, LazyLock};
use std::time::Duration;
use thiserror::Error;
use toml::{Table, Value};

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
    LazyLock::new(|| NamespacedKey::new("_", "undefined"));

impl Default for ItemStack {
    fn default() -> Self {
        warn!("An default item stack is used! This is likely an error.");
        Self::new(DEFAULT_ITEM_KEY.clone())
    }
}

impl Keyed for ItemStack {
    fn key(&self) -> &NamespacedKey {
        &self.0
    }
}

impl ItemStack {
    pub fn new(name: NamespacedKey) -> Self {
        Self(name)
    }
}

#[derive(Component, Deserialize, Serialize, Clone, Debug, Eq, Hash, PartialEq)]
#[serde(transparent)]
pub struct StackCount(u8);

impl Default for StackCount {
    fn default() -> Self {
        Self(1)
    }
}

#[derive(Component, Deserialize, Serialize, Clone, Debug, Eq, Hash, PartialEq)]
#[require(ItemStack)]
pub struct RangedAmmo();

#[derive(Component, Deserialize, Serialize, Clone, Debug, Eq, Hash, PartialEq)]
#[require(ItemStack)]
pub struct Enchantments();

impl Default for Enchantments {
    fn default() -> Self {
        Self()
    }
}

#[derive(Component, Deserialize, Serialize, Clone, Debug, Eq, Hash, PartialEq)]
#[require(ItemStack)]
#[serde(transparent)]
pub struct MaxStackSize(u8);

impl Default for MaxStackSize {
    fn default() -> Self {
        Self(1)
    }
}

#[derive(Component, Deserialize, Serialize, Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct InitialItemActions {
    pub hands_click: Option<NamespacedKey>,
    pub hands_double_click: Option<NamespacedKey>,
    pub armor_click: Option<NamespacedKey>,
    pub armor_double_click: Option<NamespacedKey>,
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
        Self::Hands(Default::default())
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

pub trait ItemActionTemplate: Send + Sync {
    fn create(&self, key: NamespacedKey, config: Table) -> Result<ItemAction, Error>;
}

impl<T: (Fn(NamespacedKey, Table) -> Result<ItemAction, Error>) + Send + Sync> ItemActionTemplate
    for T
{
    fn create(&self, key: NamespacedKey, config: Table) -> Result<ItemAction, Error> {
        self(key, config)
    }
}

#[derive(SystemParam)]
pub struct ItemActionEnvironment<'w, 's> {
    commands: Commands<'w, 's>,
    spatial_query: SpatialQuery<'w, 's>,
    asset_server: Res<'w, AssetServer>,
    player: Single<'w, 's, (Entity, &'static Transform), With<Player>>,
}

pub type ItemActions = Actions<ItemAction>;

#[derive(Clone)]
#[identify(key)]
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

#[derive(Component, Deserialize, Serialize, Debug)]
#[require(ItemStack)]
#[serde(transparent)]
pub struct Weight(f32);

impl PartialEq for Weight {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 || (self.0.is_nan() && other.0.is_nan())
    }
}
impl Eq for Weight {}

pub fn sword() -> impl Bundle {
    ItemStack(embers::SWORD.clone())
}

pub fn spear() -> impl Bundle {
    ItemStack(embers::SPEAR.clone())
}

pub fn tnt() -> impl Bundle {
    ItemStack(embers::TNT.clone())
}

pub trait ItemComponent: Keyed + for<'world> UntypedCmp<EntityRef<'world>> + Send + Sync {
    fn reset_registry(&self, world: &mut World);
    fn register_prototype(&self, world: &mut World, item: NamespacedKey, value: Value);
}

impl DynamicRegistry<dyn ItemComponent> {
    pub fn register_default<C: Component + for<'de> Deserialize<'de> + Eq>(
        &mut self,
        key: NamespacedKey,
    ) -> Result<NamespacedKey, RegistryError> {
        struct DefaultItemComponent<C: Component + for<'de> Deserialize<'de> + Eq>(
            NamespacedKey,
            PhantomData<C>,
        );
        impl<C: Component + for<'de> Deserialize<'de> + Eq> Keyed for DefaultItemComponent<C> {
            fn key(&self) -> &NamespacedKey {
                &self.0
            }
        }
        impl<C: Component + for<'de> Deserialize<'de> + Eq>
            UntypedPartialCmp<EntityRef<'_>, EntityRef<'_>> for DefaultItemComponent<C>
        {
            fn eq(&self, lhs: EntityRef<'_>, rhs: EntityRef<'_>) -> bool {
                match (lhs.get::<C>(), rhs.get::<C>()) {
                    (Some(lhs), Some(rhs)) => *lhs == *rhs,
                    (None, None) => true,
                    _ => false,
                }
            }
        }
        impl<C: Component + for<'de> Deserialize<'de> + Eq> UntypedCmp<EntityRef<'_>>
            for DefaultItemComponent<C>
        {
        }
        impl<C: Component + for<'de> Deserialize<'de> + Eq> ItemComponent for DefaultItemComponent<C> {
            fn reset_registry(&self, world: &mut World) {
                world.resource_mut::<Registry<C>>().clear();
            }
            fn register_prototype(&self, world: &mut World, item: NamespacedKey, value: Value) {
                world
                    .resource_mut::<Registry<C>>()
                    .register(item, C::deserialize(value).unwrap())
                    .expect("Failed to register item component prototype value");
            }
        }
        self.register_boxed_keyed(Box::new(DefaultItemComponent(key, PhantomData::<C>)))
    }
}

pub(super) fn plugin(app: &mut App) {
    app.init_registry::<Enchantments>()
        .init_registry::<InitialItemActions>()
        .init_registry::<ItemAction>()
        .init_dynamic_registry::<dyn ItemActionTemplate>()
        .init_registry::<MaxStackSize>()
        .init_registry::<RangedAmmo>()
        .init_registry::<Weight>()
        .init_dynamic_registry::<dyn ItemComponent>()
        .add_systems(
            PreStartup,
            (
                |mut item_action_templates: DynRegMut<dyn ItemActionTemplate>| {
                    (|| {
                        item_action_templates.register_boxed(
                            NamespacedKey::new_embers("melee"),
                            Box::new(|key: NamespacedKey, config: Table| {
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
                            }),
                        )?;
                        item_action_templates.register_boxed(
                            NamespacedKey::new_embers("throw"),
                            Box::new(|key, config| {
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
                                        info!("flag");
                                        let (player, transform) = **player;
                                        commands.spawn((
                                            primed_tnt(asset_server),
                                            exclude_source(player),
                                            *transform,
                                            LinearVelocity(transform.rotation * -Vec3::Z * velocity),
                                        ));
                                    },
                                    move |_environment, _item, _duration| next_action.clone(),
                                    ItemActionWield::Hands(action.wield),
                                    Duration::from_secs_f32(action.timeout_secs),
                                ))
                            }),
                        )?;
                        item_action_templates.register_boxed(
                            NamespacedKey::new_embers("charged_throw"),
                            Box::new(|key: NamespacedKey, config: Table| {
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
                            }),
                        )?;
                        Ok::<(), RegistryError>(())
                    })()
                    .expect("Failed to register item action templates");
                },
                |mut item_components: DynRegMut<dyn ItemComponent>| {
                    (|| {
                        item_components.register_default::<RangedAmmo>(
                            NamespacedKey::new_embers("ranged_ammo"),
                        )?;
                        item_components.register_default::<Enchantments>(
                            NamespacedKey::new_embers("enchantments"),
                        )?;
                        item_components.register_default::<InitialItemActions>(
                            NamespacedKey::new_embers("initial_actions"),
                        )?;
                        item_components.register_default::<MaxStackSize>(
                            NamespacedKey::new_embers("max_stack_size"),
                        )?;
                        item_components
                            .register_default::<Weight>(NamespacedKey::new_embers("weight"))?;
                        Ok::<(), RegistryError>(())
                    })()
                    .expect("Failed to register item components")
                },
            ),
        );
}
