use crate::utils::NamespacedKey;
use bevy::prelude::*;

pub mod embers {
    macro_rules! item {
        ($id: ident, $key: expr) => {
            pub static $id: std::sync::LazyLock<$crate::utils::NamespacedKey> =
                std::sync::LazyLock::new(|| $crate::utils::NamespacedKey::new_embers($key));
        };
    }
    item!(SWORD, "sword");
}

#[derive(Clone, Component)]
pub struct ItemStack {
    id: NamespacedKey,
    count: u8,
}

impl ItemStack {
    pub fn new(id: NamespacedKey, count: u8) -> Self {
        Self { id, count }
    }
}

#[derive(Component)]
pub struct RangedAmmo();

#[derive(Component)]
pub struct Enchantments();
impl Default for Enchantments {
    fn default() -> Self {
        Self()
    }
}

#[derive(Component)]
pub struct MaxStackSize(u8);
impl Default for MaxStackSize {
    fn default() -> Self {
        Self(1)
    }
}

#[derive(Clone, Copy, Default, Eq, Hash, PartialEq)]
pub enum ItemActionTrigger {
    #[default]
    Click,
    DoubleClick,
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub enum ItemActionSlot {
    Armor,
    Hands(HandActionWield),
}

impl Default for ItemActionSlot {
    fn default() -> Self {
        Self::Hands(HandActionWield::default())
    }
}

#[derive(Clone, Copy, Default, Eq, Hash, PartialEq)]
pub enum HandActionWield {
    #[default]
    Single,
    Dual,
}

#[derive(Component)]
pub struct ItemAction {
    on_begin: fn(),
    on_end: fn(),
    trigger: ItemActionTrigger,
    slot: ItemActionSlot,
    duration: f32,
}

impl Default for ItemAction {
    fn default() -> Self {
        Self {
            on_begin: || (),
            on_end: || (),
            trigger: ItemActionTrigger::default(),
            slot: ItemActionSlot::default(),
            duration: f32::INFINITY,
        }
    }
}

#[derive(Component)]
pub struct Weight(f32);

pub fn sword() -> impl Bundle {
    (
        ItemStack::new(embers::SWORD.clone(), 1),
        Enchantments::default(),
    )
}
