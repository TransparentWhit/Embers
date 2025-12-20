pub mod inventory;

use crate::registry::{DynamicRegistry, Registry, RegistryError};
use crate::utils::{Keyed, NamespacedKey};
use avian3d::prelude::*;
use bevy::prelude::*;
use embers_macros::identify;
use std::marker::PhantomData;
use std::time::Duration;

pub mod embers {
    macro_rules! item {
        ($id: ident, $key: expr) => {
            pub static $id: std::sync::LazyLock<$crate::utils::NamespacedKey> =
                std::sync::LazyLock::new(|| $crate::utils::NamespacedKey::new_embers($key));
        };
    }
    item!(SPEAR, "spear");
    item!(SWORD, "sword");
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

#[derive(Component, Clone, Debug, Eq, Hash, PartialEq)]
pub struct RangedAmmo();

#[derive(Component, Clone, Debug, Eq, Hash, PartialEq)]
pub struct Enchantments();

impl Default for Enchantments {
    fn default() -> Self {
        Self()
    }
}

#[derive(Component, Clone, Debug, Eq, Hash, PartialEq)]
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

pub type ItemActionEnvironment<
    'action,
    'commands_world,
    'commands_state,
    'spatial_query_world,
    'spatial_query_state,
> = (
    &'action mut Commands<'commands_world, 'commands_state>,
    &'action SpatialQuery<'spatial_query_world, 'spatial_query_state>,
    &'action Transform,
);

#[identify(key)]
pub struct ItemAction {
    key: NamespacedKey,
    pub on_begin: fn(&mut ItemActionEnvironment),
    pub on_end: fn(&mut ItemActionEnvironment, Option<Duration>),
    pub trigger: ItemActionTrigger,
    pub wield: ItemActionWield,
    pub duration: Duration,
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

#[derive(Component, Debug, PartialEq)]
pub struct Weight(f32);

/*pub fn melee(shape: &Collider) -> impl Fn(ItemActionEnvironment) {
    let filter = SpatialQueryFilter::from_mask();
    move |(spatial_query, transform)| {
        spatial_query.shape_intersections(shape, transform.translation, transform.rotation, &filter)
    }
}*/

pub fn sword() -> impl Bundle {
    (
        ItemStack(embers::SWORD.clone()),
        ItemActions::new([ItemAction {
            key: NamespacedKey::new_embers("sword_attack"),
            on_begin: |_environment| {},
            on_end: |(_commands, spatial_query, transform), duration| {
                //spatial_query.cast_shape(, transform.translation, transform.rotation, )
                println!("ended {:?}", duration);
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
                on_begin: |_environment| {
                    println!("started");
                },
                on_end: |(_commands, spatial_query, transform), duration| {
                    //spatial_query.cast_shape(, transform.translation, transform.rotation, )
                    println!("ended {:?}", duration);
                },
                trigger: ItemActionTrigger::Click,
                wield: ItemActionWield::Hands(HandActionWield::Single),
                duration: Duration::from_millis(500),
            },
            ItemAction {
                key: NamespacedKey::new_embers("spear_throw"),
                on_begin: |_environment| {},
                on_end: |(commands, _spatial_query, transform), duration| {
                    if duration.is_none() {
                        println!("Throwing spear into the console");
                        commands.spawn(());
                    }
                },
                trigger: ItemActionTrigger::DoubleClick,
                wield: ItemActionWield::Hands(HandActionWield::Single),
                duration: Duration::from_millis(500),
            },
        ]),
    )
}

pub trait ItemComponent: Send + Sync {
    fn can_stack(&self, lhs: EntityRef, rhs: EntityRef) -> bool;
}

impl DynamicRegistry<dyn ItemComponent> {
    pub fn register_default<C: Component + PartialEq>(
        &mut self,
        key: NamespacedKey,
    ) -> Result<(), RegistryError> {
        struct DefaultItemComponent<C: Component + PartialEq>(PhantomData<C>);
        impl<C: Component + PartialEq> ItemComponent for DefaultItemComponent<C> {
            fn can_stack(&self, lhs: EntityRef, rhs: EntityRef) -> bool {
                match (lhs.get::<C>(), rhs.get::<C>()) {
                    (Some(lhs), Some(rhs)) => lhs == rhs,
                    (None, None) => true,
                    _ => false,
                }
            }
        }
        self.register(key, &DefaultItemComponent(PhantomData::<C>))
    }
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<DynamicRegistry<dyn ItemComponent>>();
    app.add_systems(
        Startup,
        |mut item_component_registry: ResMut<DynamicRegistry<dyn ItemComponent>>| {
            struct StackSizeItemComponent;
            impl ItemComponent for StackSizeItemComponent {
                fn can_stack(&self, _lhs: EntityRef, _rhs: EntityRef) -> bool {
                    true
                }
            }
            item_component_registry
                .register(
                    NamespacedKey::new_embers("stack_size"),
                    &StackSizeItemComponent,
                )
                .expect("Could not register `stack_size` to item components");
            item_component_registry
                .register_default::<RangedAmmo>(NamespacedKey::new_embers("ranged_ammo"))
                .expect("Could not register `ranged_ammo` to item components");
            item_component_registry
                .register_default::<Enchantments>(NamespacedKey::new_embers("enchantments"))
                .expect("Could not register `enchantments` to item components");
            item_component_registry
                .register_default::<MaxStackSize>(NamespacedKey::new_embers("max_stack_size"))
                .expect("Could not register `max_stack_size` to item components");
            item_component_registry
                .register_default::<ItemActions>(NamespacedKey::new_embers("actions"))
                .expect("Could not register `actions` to item components");
            item_component_registry
                .register_default::<Weight>(NamespacedKey::new_embers("weight"))
                .expect("Could not register `weight` to item components");
        },
    );
    app.init_resource::<Registry<ItemAction>>();
}
