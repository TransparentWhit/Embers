use crate::utils::{Keyed, NamespacedKey};
use embers_macros::identify;
use std::collections::HashSet;

pub mod embers {
    macro_rules! attribute {
        ($id: ident, $key: expr) => {
            pub static $id: std::sync::LazyLock<$crate::utils::NamespacedKey> =
                std::sync::LazyLock::new(|| $crate::utils::NamespacedKey::new_embers($key));
        };
    }
    attribute!(MAX_HEALTH, "max_health");
    attribute!(MOVEMENT_SPEED, "movement_speed");
}

#[identify(key)]
pub struct AttributeInstance {
    pub(super) key: NamespacedKey,
    pub base: f32,
    pub modifiers: HashSet<AttributeModifier>,
}
impl AttributeInstance {
    pub fn value(&self) -> f32 {
        let mut base = self.base;
        let mut multiplier = 1f32;
        for modifier in &self.modifiers {
            match modifier.modification {
                AttributeModification::AddValue(value) => base += value,
                AttributeModification::AddMultipliedValue(multipled_value) => {
                    multiplier *= 1f32 + multipled_value
                }
            }
        }
        base * multiplier
    }
}
impl Keyed for AttributeInstance {
    fn key(&self) -> &NamespacedKey {
        &self.key
    }
}

#[identify(key)]
pub struct AttributeModifier {
    key: NamespacedKey,
    modification: AttributeModification,
}
impl AttributeModifier {
    pub fn new(key: NamespacedKey, modification: AttributeModification) -> Self {
        Self { key, modification }
    }
    pub fn modification(&self) -> &AttributeModification {
        &self.modification
    }
}
impl Keyed for AttributeModifier {
    fn key(&self) -> &NamespacedKey {
        &self.key
    }
}

pub enum AttributeModification {
    AddValue(f32),
    AddMultipliedValue(f32),
}
