use crate::pld::manager::{PayloadManager, inject_keyed_embers_payload_batch, resolve_payload};
use crate::pld::{Boxed, BoxedPayloadMarker, Payload};
use crate::utils::{Keyed, NamespacedKey, TypeKey};
use bevy::asset::AssetPath;
use bevy::ecs::template::TemplateContext;
use bevy::prelude::*;
use derive_where::derive_where;
use embers_macros::{TypeKey, identify};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::LazyLock;

pub trait AttributeType: Keyed + Send + Sync + 'static {
    fn dyn_clone(&self) -> Box<dyn AttributeType>;
    fn insert_attribute(&self, entity: &mut EntityWorldMut, actor_key: &NamespacedKey);
}

#[derive(TypePath)]
#[doc(hidden)]
pub enum DynAttributeType {}

impl BoxedPayloadMarker for DynAttributeType {
    fn payload_root() -> AssetPath<'static> {
        "attributes".into()
    }
}

pub type BoxedAttributeType = Boxed<DynAttributeType, dyn AttributeType>;

#[derive_where(Clone)]
pub struct StandardAttributeType<A: TypeKey + 'static> {
    _marker: PhantomData<fn() -> A>,
}

impl<A: TypeKey + 'static> Keyed for StandardAttributeType<A> {
    fn key(&self) -> &NamespacedKey {
        A::key()
    }
}

impl<A: TypeKey + 'static> AttributeType for StandardAttributeType<A> {
    fn dyn_clone(&self) -> Box<dyn AttributeType> {
        Box::new(self.clone())
    }
    fn insert_attribute(&self, entity: &mut EntityWorldMut, actor_key: &NamespacedKey) {
        entity.insert(
            match resolve_payload(
                entity.resource::<PayloadManager>(),
                entity.resource::<AssetServer>(),
                entity.resource::<Assets<AttributeBase>>(),
                actor_key,
            )
            .and_then(|base| base.0.get(A::key()))
            {
                Some(base) => Attributes::<Self>::new(*base),
                None => Attributes::new_virtual(),
            },
        );
    }
}

impl<A: TypeKey + 'static> StandardAttributeType<A> {
    pub fn new() -> Box<dyn AttributeType> {
        Box::new(Self {
            _marker: PhantomData,
        })
    }
}

#[derive(Clone, Component, Debug)]
pub struct Attributes<A: AttributeType> {
    base: f32,
    modifiers: HashMap<NamespacedKey, AttributeModifier>,
    _marker: PhantomData<fn() -> A>,
}

impl<A: AttributeType> Attributes<A> {
    pub fn new(base: f32) -> Self {
        Self {
            base,
            modifiers: default(),
            _marker: PhantomData,
        }
    }
    /// Creates a new virtual attribute instance.
    ///
    /// A virtual attribute instance is one that does not have a base value.
    /// Getting the [value](Self::value) of a "virtual" attribute instance is undefined behavior.
    #[inline]
    pub fn new_virtual() -> Self {
        Self::new(f32::NAN)
    }
    #[inline]
    pub fn value(&self) -> f32 {
        self.value_for(self.base)
    }
    pub fn value_for(&self, mut base: f32) -> f32 {
        let mut multiplier = 1f32;
        for modifier in self.modifiers.values() {
            match modifier.modification {
                AttributeModification::AddValue(value) => base += value,
                AttributeModification::AddMultipliedValue(multiplied_value) => {
                    multiplier *= 1. + multiplied_value
                }
            }
        }
        base * multiplier
    }
    pub fn add_modifier(&mut self, modifier: AttributeModifier) {
        self.modifiers.insert(modifier.key().clone(), modifier);
    }
    pub fn remove_modifier(&mut self, key: &NamespacedKey) {
        self.modifiers.remove(key);
    }
    pub fn with_modifiers(
        mut self,
        modifiers: impl IntoIterator<Item = AttributeModifier>,
    ) -> Self {
        for modifier in modifiers.into_iter() {
            self.add_modifier(modifier);
        }
        self
    }
}

#[derive(Clone, Debug)]
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

#[derive(Clone, Copy, Debug)]
pub enum AttributeModification {
    AddValue(f32),
    AddMultipliedValue(f32),
}

#[derive(Asset, TypePath)]
pub struct AttributeBase(HashMap<NamespacedKey, f32>);

impl AttributeBase {
    pub fn new(base: HashMap<NamespacedKey, f32>) -> Self {
        Self(base)
    }
}

impl Payload for AttributeBase {
    fn payload_root() -> AssetPath<'static> {
        "attribute_bases".into()
    }
}

pub struct AttributesTemplate {
    actor_key: NamespacedKey,
}

static DEFAULT_ATTRIBUTE_KEY: LazyLock<NamespacedKey> =
    LazyLock::new(|| NamespacedKey::new("_", "missingno"));

impl Default for AttributesTemplate {
    fn default() -> Self {
        Self::new(DEFAULT_ATTRIBUTE_KEY.clone())
    }
}

impl AttributesTemplate {
    pub fn new(actor_key: NamespacedKey) -> Self {
        Self { actor_key }
    }
}

impl Template for AttributesTemplate {
    type Output = ();
    fn build_template(&self, context: &mut TemplateContext) -> Result<Self::Output> {
        for attribute in context
            .resource::<Assets<BoxedAttributeType>>()
            .iter()
            .map(|(_id, attribute)| attribute.dyn_clone())
            .collect::<Box<_>>()
        {
            attribute.insert_attribute(context.entity, &self.actor_key);
        }
        Ok(())
    }
    fn clone_template(&self) -> Self {
        Self {
            actor_key: self.actor_key.clone(),
        }
    }
}

#[doc(hidden)]
#[derive(TypeKey)]
#[type_key = "embers:damage_taken"]
pub enum DamageTakenAttribute {}

pub type DamageTaken = StandardAttributeType<DamageTakenAttribute>;

#[doc(hidden)]
#[derive(TypeKey)]
#[type_key = "embers:knockback_taken"]
pub enum KnockbackTakenAttribute {}

pub type KnockbackTaken = StandardAttributeType<KnockbackTakenAttribute>;

#[doc(hidden)]
#[derive(TypeKey)]
#[type_key = "embers:max_health"]
pub enum MaxHealthAttribute {}

pub type MaxHealth = StandardAttributeType<MaxHealthAttribute>;

#[doc(hidden)]
#[derive(TypeKey)]
#[type_key = "embers:melee_damage"]
pub enum MeleeDamageAttribute {}

pub type MeleeDamage = StandardAttributeType<MeleeDamageAttribute>;

#[doc(hidden)]
#[derive(TypeKey)]
#[type_key = "embers:movement_speed"]
pub enum MovementSpeedAttribute {}

pub type MovementSpeed = StandardAttributeType<MovementSpeedAttribute>;

pub(super) fn plugin(app: &mut App) {
    app.init_asset::<BoxedAttributeType>()
        .init_asset::<AttributeBase>()
        .add_systems(
            PreStartup,
            inject_keyed_embers_payload_batch::<BoxedAttributeType>(
                "{}",
                [
                    DamageTaken::new(),
                    KnockbackTaken::new(),
                    MaxHealth::new(),
                    MeleeDamage::new(),
                    MovementSpeed::new(),
                ],
            ),
        );
}
