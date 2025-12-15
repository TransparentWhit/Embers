pub mod inventory;

use crate::registry::{DynamicRegistry, RegistryError};
use crate::utils::NamespacedKey;
use bevy::prelude::*;
use embers_macros::identify;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::time::Duration;

pub mod embers {
    macro_rules! item {
        ($id: ident, $key: expr) => {
            pub static $id: std::sync::LazyLock<$crate::utils::NamespacedKey> =
                std::sync::LazyLock::new(|| $crate::utils::NamespacedKey::new_embers($key));
        };
    }
    item!(SWORD, "sword");
}

#[derive(Component, Clone, Debug, Eq, Hash, PartialEq)]
#[require(StackCount)]
pub struct ItemStack(NamespacedKey);

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

#[derive(Component, Clone, Debug)]
#[identify(key)]
pub struct ItemAction {
    key: NamespacedKey,
    pub on_begin: fn(),
    pub on_end: fn(),
    pub trigger: ItemActionTrigger,
    pub wield: ItemActionWield,
    pub duration: Duration,
}

#[derive(Component, Clone, Debug, Eq, PartialEq)]
pub struct ItemActions(HashMap<ItemActionSlot, ItemAction>);

impl ItemActions {
    pub fn new(actions: impl IntoIterator<Item = ItemAction>) -> Self {
        Self(HashMap::from_iter(
            actions
                .into_iter()
                .map(|action| (action.wield.slot(), action)),
        ))
    }
    pub fn get(&self, slot: ItemActionSlot) -> Option<&ItemAction> {
        self.0.get(&slot)
    }
}

#[derive(Component, Debug)]
pub struct Weight(f32);

pub fn sword() -> impl Bundle {
    (
        ItemStack(embers::SWORD.clone()),
        Enchantments::default(),
        ItemActions::new([ItemAction {
            key: NamespacedKey::new_embers("sword_attack_0"),
            on_begin: || {
                println!("started");
            },
            on_end: || {
                println!("ended");
            },
            trigger: ItemActionTrigger::Click,
            wield: ItemActionWield::Hands(HandActionWield::Single),
            duration: Duration::from_millis(500),
        }]),
    )
}

pub trait ItemComponent: Send + Sync {
    fn can_stack(&self, a: EntityRef, b: EntityRef) -> bool;
}

impl DynamicRegistry<dyn ItemComponent> {
    pub fn register_default<C: Component + PartialEq>(
        &mut self,
        key: NamespacedKey,
    ) -> Result<(), RegistryError> {
        struct DefaultItemComponent<C: Component + PartialEq>(PhantomData<C>);
        impl<C: Component + PartialEq> ItemComponent for DefaultItemComponent<C> {
            fn can_stack(&self, a: EntityRef, b: EntityRef) -> bool {
                match (a.get::<C>(), b.get::<C>()) {
                    (Some(a), Some(b)) => a == b,
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
}
