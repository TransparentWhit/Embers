use crate::pld::{Boxed, PayloadManager, inject_keyed_embers_payload_batch, resolve_payload};
use crate::utils::{Keyed, NamespacedKey, Void};
use bevy::ecs::template::TemplateContext;
use bevy::prelude::*;
use derive_where::derive_where;
use embers_macros::identify;
use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;
use std::sync::LazyLock;
use thiserror::Error;

pub trait Attribute: Keyed + Send + Sync + 'static {
    fn dyn_clone(&self) -> Box<dyn Attribute>;
    fn insert_attribute(&self, entity: &mut EntityWorldMut, actor_key: &NamespacedKey);
}

#[derive(TypePath)]
#[doc(hidden)]
pub enum DynAttribute {}

pub type BoxedAttribute = Boxed<DynAttribute, dyn Attribute>;

#[derive_where(Clone)]
pub struct StandardAttribute<A: 'static> {
    key: NamespacedKey,
    _marker: PhantomData<fn() -> A>,
}

impl<A: 'static> Keyed for StandardAttribute<A> {
    fn key(&self) -> &NamespacedKey {
        &self.key
    }
}

impl<A: 'static> Attribute for StandardAttribute<A> {
    fn dyn_clone(&self) -> Box<dyn Attribute> {
        Box::new(self.clone())
    }
    fn insert_attribute(&self, entity: &mut EntityWorldMut, actor_key: &NamespacedKey) {
        entity.insert(
            match resolve_payload(
                entity.resource::<PayloadManager>(),
                entity.resource::<AssetServer>(),
                entity.resource::<Assets<AttributeBase>>(),
                format!("attribute_bases/{}", self.key.path_string()),
            )
            .unwrap()
            .0
            .get(actor_key)
            {
                Some(base) => Attributes::<Self>::new(*base),
                None => Attributes::new_virtual(),
            },
        );
    }
}

impl<A: 'static> StandardAttribute<A> {
    pub fn new(key: NamespacedKey) -> Box<dyn Attribute> {
        Box::new(Self {
            key,
            _marker: PhantomData,
        })
    }
}

#[derive(Component, Debug)]
pub struct Attributes<A: Attribute> {
    base: f32,
    modifiers: HashSet<AttributeModifier>,
    _marker: PhantomData<fn() -> A>,
}

impl<A: Attribute> Attributes<A> {
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
        for modifier in &self.modifiers {
            match modifier.modification {
                AttributeModification::AddValue(value) => base += value,
                AttributeModification::AddMultipliedValue(multiplied_value) => {
                    multiplier *= 1. + multiplied_value
                }
            }
        }
        base * multiplier
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

#[derive(Asset, TypePath)]
pub struct AttributeBase(HashMap<NamespacedKey, f32>);

impl AttributeBase {
    pub fn new(base: HashMap<NamespacedKey, f32>) -> Self {
        Self(base)
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

#[derive(Debug, Error)]
#[error("Attributes have been inserted")]
struct AttributesInserted;

impl Template for AttributesTemplate {
    type Output = Void;
    fn build_template(&self, context: &mut TemplateContext) -> Result<Self::Output> {
        for attribute in context
            .resource::<Assets<BoxedAttribute>>()
            .iter()
            .map(|(_id, attribute)| attribute.dyn_clone())
            .collect::<Box<_>>()
        {
            attribute.insert_attribute(context.entity, &self.actor_key);
        }
        Err(BevyError::ignore(AttributesInserted))
    }
    fn clone_template(&self) -> Self {
        Self {
            actor_key: self.actor_key.clone(),
        }
    }
}

#[doc(hidden)]
pub enum MaxHealthAttribute {}

pub type MaxHealth = StandardAttribute<MaxHealthAttribute>;

#[doc(hidden)]
pub enum MovementSpeedAttribute {}

pub type MovementSpeed = StandardAttribute<MovementSpeedAttribute>;

pub(super) fn plugin(app: &mut App) {
    app.init_asset::<BoxedAttribute>()
        .init_asset::<AttributeBase>()
        .add_systems(
            PreStartup,
            inject_keyed_embers_payload_batch::<BoxedAttribute>(
                "attributes/{}",
                [
                    MaxHealth::new(NamespacedKey::new_embers("max_health")),
                    MovementSpeed::new(NamespacedKey::new_embers("movement_speed")),
                ],
            ),
        );
}
