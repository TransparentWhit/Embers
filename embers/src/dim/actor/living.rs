pub mod attributes;
pub mod creeper;
pub mod dummy;
pub mod player;

use super::actor;
use crate::dim::CollisionLayer;
use crate::dim::actor::living::attributes::AttributeInstance;
use crate::utils::NamespacedKey;
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_tnua::prelude::*;
use std::collections::HashMap;

#[derive(Component, Debug)]
pub struct Health(pub f32);

#[derive(Component, Debug)]
pub struct Attributes(pub HashMap<NamespacedKey, AttributeInstance>);

const LOCKED_AXES: LockedAxes = LockedAxes::new().lock_rotation_x().lock_rotation_z();

pub fn living_actor(attributes: &HashMap<NamespacedKey, f32>) -> impl Bundle {
    (
        actor(),
        Health(
            *attributes
                .get(&attributes::embers::MAX_HEALTH)
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
