pub mod attributes;
pub mod creeper;
pub mod dummy;
pub mod player;

use super::actor;
use crate::dim::CollisionLayer;
use crate::dim::actor::living::attributes::AttributeInstance;
use crate::reg::{RegistryAccess, RegistryInitExt};
use crate::utils::NamespacedKey;
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_tnua::prelude::*;
use std::collections::HashMap;

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

const LOCKED_AXES: LockedAxes = LockedAxes::new().lock_rotation_x().lock_rotation_z();

pub fn living_actor(
    key: &NamespacedKey,
    attribute_bases: impl RegistryAccess<Item = AttributeBase>,
) -> impl Bundle {
    let attributes: HashMap<NamespacedKey, AttributeInstance> = attribute_bases
        .get(key)
        .expect(&format!(
            "Attribute base does not exist for actor '{}'",
            key
        ))
        .0
        .iter()
        .map(|(key, base)| (key.clone(), AttributeInstance::new(key.clone(), *base)))
        .collect();
    (
        actor(),
        Health(
            attributes
                .get(&attributes::embers::MAX_HEALTH)
                .map(|attribute_instance| attribute_instance.value())
                .unwrap_or(0.),
        ),
        Attributes(attributes),
        RigidBody::Dynamic,
        CollisionLayers::new(
            CollisionLayer::LivingActor,
            [
                CollisionLayer::LivingActor,
                CollisionLayer::MiscellaneousActor,
                CollisionLayer::Environment,
            ],
        ),
        LOCKED_AXES,
        TnuaController::default(),
    )
}

pub(super) fn plugin(app: &mut App) {
    app.init_registry::<AttributeBase>();
}
