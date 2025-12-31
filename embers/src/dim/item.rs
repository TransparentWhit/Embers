pub mod inv;

use crate::dim::actor::living::player::PlayerFacing;
use crate::dim::actor::primed_tnt::primed_tnt;
use crate::reg::{DynRegMut, DynamicRegistry, Registry, RegistryError, RegistryInitExt};
use crate::utils::{Keyed, NamespacedKey, UntypedCmp, UntypedPartialCmp};
use anyhow::Error;
use avian3d::prelude::*;
use bevy::prelude::*;
use embers_macros::identify;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::sync::{Arc, LazyLock};
use std::time::Duration;
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
    pub fn get(&self, slot: ItemActionSlot, trigger: ItemActionTrigger) -> Option<&NamespacedKey> {
        match (slot, trigger) {
            (ItemActionSlot::Hands, ItemActionTrigger::Click) => self.hands_click.as_ref(),
            (ItemActionSlot::Hands, ItemActionTrigger::DoubleClick) => {
                self.hands_double_click.as_ref()
            }
            (ItemActionSlot::Armor, ItemActionTrigger::Click) => self.armor_click.as_ref(),
            (ItemActionSlot::Armor, ItemActionTrigger::DoubleClick) => {
                self.armor_double_click.as_ref()
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum ItemActionTrigger {
    #[default]
    Click,
    DoubleClick,
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

pub type ItemActionEnvironment<'action, 'cmd_world, 'cmd_state, 'sq_world, 'sq_state> = (
    &'action mut Commands<'cmd_world, 'cmd_state>,
    &'action SpatialQuery<'sq_world, 'sq_state>,
    &'action AssetServer,
    &'action Transform,
    &'action PlayerFacing,
);

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

#[derive(Clone)]
#[identify(key)]
pub struct ItemAction {
    key: NamespacedKey,
    pub on_begin: Arc<dyn Fn(&mut ItemActionEnvironment) + Send + Sync>,
    pub on_end: Arc<
        dyn Fn(&mut ItemActionEnvironment, Option<Duration>) -> Option<NamespacedKey> + Send + Sync,
    >,
    pub trigger: ItemActionTrigger,
    pub wield: ItemActionWield,
    pub duration: Duration,
}

impl Keyed for ItemAction {
    fn key(&self) -> &NamespacedKey {
        &self.key
    }
}

#[derive(Default, Eq, PartialEq, Clone)]
pub struct SlotItemActions(
    /// Click
    Option<ItemAction>,
    /// Double click
    Option<ItemAction>,
);

impl SlotItemActions {
    pub fn get(&self, trigger: ItemActionTrigger) -> Option<&ItemAction> {
        match trigger {
            ItemActionTrigger::Click => self.0.as_ref(),
            ItemActionTrigger::DoubleClick => self.1.as_ref(),
        }
    }
    pub fn get_mut(&mut self, trigger: ItemActionTrigger) -> Option<&mut ItemAction> {
        match trigger {
            ItemActionTrigger::Click => self.0.as_mut(),
            ItemActionTrigger::DoubleClick => self.1.as_mut(),
        }
    }
    pub fn set(&mut self, trigger: ItemActionTrigger, action: ItemAction) {
        match trigger {
            ItemActionTrigger::Click => self.0 = Some(action),
            ItemActionTrigger::DoubleClick => self.1 = Some(action),
        }
    }
    pub fn clear(&mut self, trigger: ItemActionTrigger) {
        match trigger {
            ItemActionTrigger::Click => self.0 = None,
            ItemActionTrigger::DoubleClick => self.1 = None,
        }
    }
}

#[derive(Component, Eq, PartialEq)]
pub struct ItemActions(
    /// Hands
    SlotItemActions,
    /// Armor
    SlotItemActions,
);

impl ItemActions {
    pub fn new(actions: impl IntoIterator<Item = ItemAction>) -> Self {
        let mut hands_actions = SlotItemActions::default();
        let mut armor_actions = SlotItemActions::default();
        for action in actions.into_iter() {
            match action.wield.slot() {
                ItemActionSlot::Hands => &mut hands_actions,
                ItemActionSlot::Armor => &mut armor_actions,
            }
            .set(action.trigger, action);
        }
        Self(hands_actions, armor_actions)
    }
    pub fn get(&self, slot: ItemActionSlot) -> &SlotItemActions {
        match slot {
            ItemActionSlot::Hands => &self.0,
            ItemActionSlot::Armor => &self.1,
        }
    }
    pub fn get_mut(&mut self, slot: ItemActionSlot) -> &mut SlotItemActions {
        match slot {
            ItemActionSlot::Hands => &mut self.0,
            ItemActionSlot::Armor => &mut self.1,
        }
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
    (
        ItemStack(embers::SPEAR.clone()),
        /*ItemActions::new([
            ItemAction {
                key: NamespacedKey::new_embers("spear_attack_0"),
                on_begin: Arc::new(|_environment| {
                    println!("started");
                }),
                on_end: Arc::new(
                    |(_commands, spatial_query, _asset_server, transform, _player_facing),
                     duration| {
                        //spatial_query.cast_shape(, transform.translation, transform.rotation, )
                        println!("ended {:?}", duration);
                        None
                    },
                ),
                trigger: ItemActionTrigger::Click,
                wield: ItemActionWield::Hands(HandActionWield::Single),
                duration: Duration::from_millis(500),
            },
            ItemAction {
                key: NamespacedKey::new_embers("spear_throw"),
                on_begin: &|_environment| {},
                on_end: &|(commands, _spatial_query, _asset_server, transform, _player_facing),
                          duration| {
                    if duration.is_none() {
                        println!("Throwing spear into the console");
                        commands.spawn(());
                    }
                    None
                },
                trigger: ItemActionTrigger::DoubleClick,
                wield: ItemActionWield::Hands(HandActionWield::Single),
                duration: Duration::from_millis(500),
            },
        ]),*/
    )
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
    ) -> Result<(), RegistryError> {
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
                world.insert_resource(Registry::<C>::new());
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
                                    arc: f32,
                                    range: f32,
                                    wield: HandActionWield,
                                    duration: f32,
                                    next_action: Option<String>,
                                }
                                let action = Melee::deserialize(config)?;
                                let next_action = action.next_action.as_ref().and_then(|next| {
                                    NamespacedKey::try_from_with_namespaced(next.as_str(), &key)
                                        .ok()
                                });
                                Ok(ItemAction {
                                    on_begin: Arc::new(|_environment| {}),
                                    on_end: Arc::new(
                                        move |(
                                            _commands,
                                            spatial_query,
                                            _asset_server,
                                            transform,
                                            _player_facing,
                                        ),
                                              duration| {
                                            if duration.is_none() {
                                                // todo
                                                //spatial_query.cast_shape();
                                                println!("melee");
                                                next_action.clone()
                                            } else {
                                                None
                                            }
                                        },
                                    ),
                                    trigger: ItemActionTrigger::Click,
                                    wield: ItemActionWield::Hands(action.wield),
                                    duration: Duration::from_secs_f32(action.duration),
                                    key,
                                })
                            }),
                        )?;
                        item_action_templates.register_boxed(
                            NamespacedKey::new_embers("throw"),
                            Box::new(|key, config| {
                                #[derive(Deserialize)]
                                struct Throw {
                                    wield: HandActionWield,
                                    timeout: f32,
                                    next_action: Option<String>,
                                }
                                let action = Throw::deserialize(config)?;
                                let next_action = action.next_action.as_ref().and_then(|next| {
                                    NamespacedKey::try_from_with_namespaced(next.as_str(), &key)
                                        .ok()
                                });
                                Ok(ItemAction {
                                    on_begin: Arc::new(|_environment| {}),
                                    on_end: Arc::new(
                                        move |(
                                            commands,
                                            _spatial_query,
                                            asset_server,
                                            transform,
                                            player_facing,
                                        ),
                                              _duration| {
                                            commands.spawn((
                                                primed_tnt(asset_server),
                                                **transform,
                                                LinearVelocity(player_facing.0.as_vec3() * 6.),
                                            ));
                                            next_action.clone()
                                        },
                                    ),
                                    trigger: ItemActionTrigger::Click,
                                    wield: ItemActionWield::Hands(action.wield),
                                    duration: Duration::from_secs_f32(action.timeout),
                                    key,
                                })
                            }),
                        )?;
                        item_action_templates.register_boxed(
                            NamespacedKey::new_embers("charged_throw"),
                            Box::new(|key: NamespacedKey, config: Table| {
                                #[derive(Deserialize)]
                                struct ChargedThrow {
                                    wield: HandActionWield,
                                    hold_threshold: Option<f32>,
                                    hold_action: Option<String>,
                                }
                                let action = ChargedThrow::deserialize(config)?;
                                let hold_action = action.hold_action.as_ref().and_then(|next| {
                                    NamespacedKey::try_from_with_namespaced(next.as_str(), &key)
                                        .ok()
                                });
                                Ok(ItemAction {
                                    on_begin: Arc::new(|_environment| {}),
                                    on_end: Arc::new(
                                        move |(
                                            _commands,
                                            spatial_query,
                                            _asset_server,
                                            transform,
                                            _player_facing,
                                        ),
                                              duration| {
                                            if duration.is_none() {
                                                hold_action.clone()
                                            } else {
                                                println!("throwing");
                                                None
                                            }
                                        },
                                    ),
                                    trigger: ItemActionTrigger::DoubleClick,
                                    wield: ItemActionWield::Hands(action.wield),
                                    duration: action
                                        .hold_threshold
                                        .map_or(Duration::MAX, Duration::from_secs_f32),
                                    key,
                                })
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
