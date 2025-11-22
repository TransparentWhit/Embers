use crate::key_identify;
use crate::utils::{Keyed, NamespacedKey};
use std::collections::HashSet;

pub mod embers {
    use crate::utils::NamespacedKey;
    use std::sync::LazyLock;
    pub static MAX_HEALTH: LazyLock<NamespacedKey> =
        LazyLock::new(|| NamespacedKey::new_embers("max_health"));
    pub static MOVEMENT_SPEED: LazyLock<NamespacedKey> =
        LazyLock::new(|| NamespacedKey::new_embers("movement_speed"));
}

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
key_identify!(AttributeInstance);

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
key_identify!(AttributeModifier);

pub enum AttributeModification {
    AddValue(f32),
    AddMultipliedValue(f32),
}
