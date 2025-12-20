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

#[derive(Debug)]
#[identify(key)]
pub struct AttributeInstance {
    key: NamespacedKey,
    base: f32,
    modifiers: HashSet<AttributeModifier>,
}

impl AttributeInstance {
    pub fn new(key: NamespacedKey, base: f32) -> Self {
        Self {
            key,
            base,
            modifiers: Default::default(),
        }
    }
    /// Creates a new virtual attribute instance.
    ///
    /// A virtual attribute instance is one that does not have a base value.
    /// Getting the [value](Self::value) of a "virtual" attribute instance is undefined behavior.
    #[inline]
    pub fn new_virtual(key: NamespacedKey) -> Self {
        Self::new(key, 1.)
    }
    #[inline]
    pub fn value(&self) -> f32 {
        self.value_for(self.base)
    }
    pub fn value_for(&self, mut base: f32) -> f32 {
        let mut multiplier = 1f32;
        for modifier in &self.modifiers {
            match modifier.modification {
                AttributeModification::AddValue(value) => base += value,
                AttributeModification::AddMultipliedValue(multipled_value) => {
                    multiplier *= 1. + multipled_value
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

#[derive(Debug)]
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

#[derive(Debug)]
pub enum AttributeModification {
    AddValue(f32),
    AddMultipliedValue(f32),
}
