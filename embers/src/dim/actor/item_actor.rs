use super::super::PhysicsPreset;
use crate::dim::actor::actor;
use crate::utils::NamespacedKey;
use avian3d::prelude::*;
use bevy::prelude::*;
use std::sync::LazyLock;

pub static KEY: LazyLock<NamespacedKey> = LazyLock::new(|| NamespacedKey::new_embers("item"));

#[derive(Component, Debug)]
pub struct ItemActor(pub Entity);

pub fn item_actor(item: Entity) -> impl Bundle {
    (
        actor(),
        PhysicsPreset::Phantom.physics(),
        Collider::cuboid(0.25, 0.25, 0.25),
        ItemActor(item),
    )
}
