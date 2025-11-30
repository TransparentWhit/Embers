pub mod attributes;
pub mod creeper;
pub mod player;

use super::entity;
use crate::utils::NamespacedKey;
use crate::world::entity::living::attributes::AttributeInstance;
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_tnua::prelude::*;
use std::collections::HashMap;

#[derive(Component)]
pub struct Health(pub f32);

#[derive(Component)]
pub struct Attributes(pub HashMap<NamespacedKey, AttributeInstance>);

const LOCKED_AXES: LockedAxes = LockedAxes::new().lock_rotation_x().lock_rotation_z();

pub fn living_entity(attributes: &HashMap<NamespacedKey, f32>) -> impl Bundle {
    (
        entity(),
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
        LOCKED_AXES,
        TnuaController::default(),
    )
}
