pub mod inv;

use crate::dim::actor::living::player::PlayerFacing;
use crate::dim::actor::primed_tnt::primed_tnt;
use crate::reg::{DynamicRegistry, Registry, RegistryError, RegistryInitExt};
use crate::utils::{Keyed, NamespacedKey, UntypedCmp, UntypedPartialCmp};
use avian3d::prelude::*;
use bevy::prelude::*;
use embers_macros::identify;
use serde::Deserialize;
use std::marker::PhantomData;
use std::time::Duration;
use toml::Value;

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

#[derive(Component, Clone, Debug, Eq, Hash, PartialEq)]
#[require(StackCount)]
pub struct ItemStack(NamespacedKey);

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

#[derive(Component, Clone, Debug, Eq, Hash, PartialEq)]
pub struct StackCount(u8);

impl Default for StackCount {
    fn default() -> Self {
        Self(1)
    }
}

#[derive(Component, Clone, Debug, Deserialize, Eq, Hash, PartialEq)]
pub struct RangedAmmo();

#[derive(Component, Clone, Debug, Deserialize, Eq, Hash, PartialEq)]
pub struct Enchantments();

impl Default for Enchantments {
    fn default() -> Self {
        Self()
    }
}

#[derive(Component, Clone, Debug, Deserialize, Eq, Hash, PartialEq)]
pub struct MaxStackSize(u8);

impl Default for MaxStackSize {
    fn default() -> Self {
        Self(1)
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

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
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

#[derive(Clone)]
#[identify(key)]
pub struct ItemAction {
    key: NamespacedKey,
    pub on_begin: &'static (dyn Fn(&mut ItemActionEnvironment) + Send + Sync),
    pub on_end: &'static (
                 dyn Fn(&mut ItemActionEnvironment, Option<Duration>) -> Option<NamespacedKey>
                     + Send
                     + Sync
             ),
    pub trigger: ItemActionTrigger,
    pub wield: ItemActionWield,
    pub duration: Duration,
}

impl Keyed for ItemAction {
    fn key(&self) -> &NamespacedKey {
        &self.key
    }
}

#[derive(Default, Eq, PartialEq)]
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
    fn set(&mut self, trigger: ItemActionTrigger, action: ItemAction) {
        match trigger {
            ItemActionTrigger::Click => self.0 = Some(action),
            ItemActionTrigger::DoubleClick => self.1 = Some(action),
        }
    }
    fn clear(&mut self, trigger: ItemActionTrigger) {
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

#[derive(Component, Debug, Deserialize)]
pub struct Weight(f32);

impl PartialEq for Weight {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 || (self.0.is_nan() && other.0.is_nan())
    }
}
impl Eq for Weight {}

pub fn sword() -> impl Bundle {
    (
        ItemStack(embers::SWORD.clone()),
        ItemActions::new([ItemAction {
            key: NamespacedKey::new_embers("sword_attack"),
            on_begin: &|_environment| {},
            on_end: &|(_commands, spatial_query, _asset_server, transform, _player_facing),
                      duration| {
                //spatial_query.cast_shape(, transform.translation, transform.rotation, )
                println!("ended {:?}", duration);
                None
            },
            trigger: ItemActionTrigger::Click,
            wield: ItemActionWield::Hands(HandActionWield::Single),
            duration: Duration::from_millis(500),
        }]),
    )
}

pub fn spear() -> impl Bundle {
    (
        ItemStack(embers::SPEAR.clone()),
        ItemActions::new([
            ItemAction {
                key: NamespacedKey::new_embers("spear_attack_0"),
                on_begin: &|_environment| {
                    println!("started");
                },
                on_end: &|(_commands, spatial_query, _asset_server, transform, _player_facing),
                          duration| {
                    //spatial_query.cast_shape(, transform.translation, transform.rotation, )
                    println!("ended {:?}", duration);
                    None
                },
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
        ]),
    )
}

pub fn tnt() -> impl Bundle {
    (
        ItemStack(embers::TNT.clone()),
        ItemActions::new([ItemAction {
            key: NamespacedKey::new_embers("tnt_throw"),
            on_begin: &|_environment| {},
            on_end: &|(commands, _spatial_query, asset_server, transform, player_facing),
                      _duration| {
                commands.spawn((
                    primed_tnt(asset_server),
                    **transform,
                    LinearVelocity(player_facing.0.as_vec3() * 3.),
                ));
                None
            },
            trigger: ItemActionTrigger::Click,
            wield: ItemActionWield::Hands(HandActionWield::Single),
            duration: Duration::from_millis(250),
        }]),
    )
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
        .init_registry::<ItemAction>()
        .init_registry::<MaxStackSize>()
        .init_registry::<RangedAmmo>()
        .init_registry::<Weight>()
        .init_dynamic_registry::<dyn ItemComponent>()
        .add_systems(
            PreStartup,
            (
                |mut item_components: ResMut<DynamicRegistry<dyn ItemComponent>>| {
                    (|| {
                        item_components.register_default::<RangedAmmo>(
                            NamespacedKey::new_embers("ranged_ammo"),
                        )?;
                        item_components.register_default::<Enchantments>(
                            NamespacedKey::new_embers("enchantments"),
                        )?;
                        item_components.register_default::<MaxStackSize>(
                            NamespacedKey::new_embers("max_stack_size"),
                        )?;
                        /*item_components
                        .register_default::<ItemActions>(NamespacedKey::new_embers("actions"))?;*/
                        item_components
                            .register_default::<Weight>(NamespacedKey::new_embers("weight"))?;
                        Ok::<(), RegistryError>(())
                    })()
                    .expect("Failed to register item components")
                },
                /*|mut enchantments: ResMut<Registry<Enchantments>>,
                 mut max_stack_size: ResMut<Registry<MaxStackSize>>,
                 mut item_actions: ResMut<Registry<ItemAction>>,
                 mut weights: ResMut<Registry<Weight>>| {
                    fn melee(
                        key: NamespacedKey,
                        next: NamespacedKey,
                        wield: HandActionWield,
                        duration: Duration,
                    ) -> ItemAction {
                        let next = next;
                        ItemAction {
                            key,
                            on_begin: &|_environment| {},
                            on_end: &move |(
                                commands,
                                spatial_query,
                                _asset_server,
                                transform,
                                player_facing,
                            ),
                                           duration| {
                                if duration.is_none() {
                                    Some((&next).clone())
                                } else {
                                    None
                                }
                            },
                            trigger: ItemActionTrigger::Click,
                            wield: ItemActionWield::Hands(wield),
                            duration,
                        }
                    }
                    item_actions.register_keyed(ItemAction {
                        key: NamespacedKey::new_embers("sword_attack"),
                        on_begin: &|_environment| {},
                        on_end: &|(
                            _commands,
                            spatial_query,
                            _asset_server,
                            transform,
                            _player_facing,
                        ),
                                  duration| {
                            //spatial_query.cast_shape(, transform.translation, transform.rotation, )
                            println!("ended {:?}", duration);
                            None
                        },
                        trigger: ItemActionTrigger::Click,
                        wield: ItemActionWield::Hands(HandActionWield::Single),
                        duration: Duration::from_millis(500),
                    });
                    item_actions.register_keyed(ItemAction {
                        key: NamespacedKey::new_embers("spear_attack_0"),
                        on_begin: &|_environment| {
                            println!("started");
                        },
                        on_end: &|(
                            _commands,
                            spatial_query,
                            _asset_server,
                            transform,
                            _player_facing,
                        ),
                                  duration| {
                            //spatial_query.cast_shape(, transform.translation, transform.rotation, )
                            println!("ended {:?}", duration);
                            None
                        },
                        trigger: ItemActionTrigger::Click,
                        wield: ItemActionWield::Hands(HandActionWield::Single),
                        duration: Duration::from_millis(500),
                    });
                },*/
            ),
        );
}
