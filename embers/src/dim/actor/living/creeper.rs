use super::{AttributeBase, living_actor};
use crate::dim::actor::MOVEMENT_CONFIG_NAMESPACE;
use crate::reg::Registry;
use crate::utils::NamespacedKey;
use avian3d::prelude::Collider;
use bevy::prelude::*;
use std::sync::LazyLock;
use uuid::Uuid;

pub static KEY: LazyLock<NamespacedKey> = LazyLock::new(|| NamespacedKey::new_embers("creeper"));

pub static UUID: LazyLock<Uuid> =
    LazyLock::new(|| Uuid::new_v5(&MOVEMENT_CONFIG_NAMESPACE, KEY.to_string().as_bytes()));

#[derive(Component, Debug)]
pub struct Creeper {}

pub fn creeper(attribute_bases: &Registry<AttributeBase>) -> impl Bundle {
    (
        living_actor(&KEY, &UUID, attribute_bases, false),
        Collider::cylinder(0.5, 1.7),
        Creeper {},
    )
}
