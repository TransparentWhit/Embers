pub mod creeper;
pub mod player;

use super::entity;
use crate::key_identify;
use crate::utils::{Keyed, NamespacedKey};
use avian3d::prelude::RigidBody;
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

#[derive(Component)]
pub struct Health(pub f32);

#[derive(Component)]
pub struct Attributes(pub HashMap<NamespacedKey, AttributeInstance>);

#[derive(Component)]
pub struct AttributeInstance {
    key: NamespacedKey,
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

#[derive(Component)]
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

#[derive(Component)]
pub enum AttributeModification {
    AddValue(f32),
    AddMultipliedValue(f32),
}

pub fn living_entity(attributes: &HashMap<NamespacedKey, f32>) -> impl Bundle {
    (
        entity(),
        Health(
            *attributes
                .get(&NamespacedKey::new("embers", "max_health"))
                .unwrap_or(&0f32),
        ),
        Attributes(
            attributes
                .iter()
                .map(|(key, base)| {
                    (
                        key.clone(),
                        AttributeInstance {
                            key: key.clone(),
                            base: *base,
                            modifiers: Default::default(),
                        },
                    )
                })
                .collect(),
        ),
        RigidBody::Kinematic,
    )
}
