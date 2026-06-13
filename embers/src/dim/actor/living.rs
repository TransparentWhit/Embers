pub mod attributes;
pub mod creeper;
pub mod dummy;
pub mod player;

use super::super::PhysicsPreset;
use super::actor;
use crate::dim::Movements;
use crate::dim::actor::living::attributes::AttributeInstance;
use crate::reg::{Registry, RegistryInitExt};
use crate::utils::NamespacedKey;
use bevy::prelude::*;
use bevy_tnua::prelude::*;
use std::collections::HashMap;
use std::marker::PhantomData;
use uuid::Uuid;

#[derive(Component, Debug)]
pub struct Health(pub f32);

#[derive(Component, Debug)]
pub struct Attributes(pub HashMap<NamespacedKey, AttributeInstance>);

pub struct AttributeBase(HashMap<NamespacedKey, f32>);

impl AttributeBase {
    pub fn new(base: HashMap<NamespacedKey, f32>) -> Self {
        Self(base)
    }
}

pub fn living_actor(
    key: &NamespacedKey,
    uuid: &Uuid,
    attribute_bases: &Registry<AttributeBase>,
    interactable: bool,
) -> impl Bundle {
    let attributes: HashMap<NamespacedKey, AttributeInstance> = attribute_bases
        .get(key)
        .expect("Attribute base not found")
        .0
        .iter()
        .map(|(key, base)| (key.clone(), AttributeInstance::new(key.clone(), *base)))
        .collect();
    (
        actor(),
        PhysicsPreset::LivingActor.physics(interactable),
        Health(
            attributes
                .get(&attributes::embers::MAX_HEALTH)
                .map(|attribute_instance| attribute_instance.value())
                .unwrap_or(0.),
        ),
        Attributes(attributes),
        TnuaController::<Movements>::default(),
        TnuaConfig::<Movements>(Handle::Uuid(uuid.clone(), PhantomData)),
    )
}

pub(super) fn plugin(app: &mut App) {
    app.init_registry::<AttributeBase>();
}
