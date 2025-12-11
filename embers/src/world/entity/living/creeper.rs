use crate::utils::NamespacedKey;
use crate::world::entity::living::living_entity;
use avian3d::prelude::Collider;
use bevy::prelude::*;
use std::collections::HashMap;
use std::sync::LazyLock;

static ATTRIBUTES: LazyLock<HashMap<NamespacedKey, f32>> =
    LazyLock::new(|| HashMap::from([(NamespacedKey::new("embers", "max_health"), 20.)]));

#[derive(Component)]
pub struct Creeper {}

pub fn creeper() -> impl Bundle {
    (
        living_entity(&ATTRIBUTES),
        Collider::cylinder(0.5, 1.7),
        Creeper {},
    )
}
