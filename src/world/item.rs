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

#[derive(Component)]
pub struct ItemStack {
    id: NamespacedKey,
    count: u8,
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

#[derive(Default)]
pub enum ItemActionTrigger {
    #[default]
    Click,
    DoubleClick,
}

#[derive(Default)]
pub enum ItemActionWield {
    #[default]
    Single,
    Dual,
}

#[derive(Component)]
pub struct ItemAction {
    on_begin: fn(),
    on_end: fn(),
    trigger: ItemActionTrigger,
    wield: ItemActionWield,
    duration: f32,
}

#[derive(Component)]
pub struct Weight(f32);

pub fn item(id: NamespacedKey, count: u8) -> impl Bundle {
    ItemStack { id, count }
}

pub fn sword() -> impl Bundle {
    (item(embers::SWORD.clone(), 1), Enchantments::default())
}
