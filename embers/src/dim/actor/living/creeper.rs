use crate::dim::actor::living::{AttributeBase, living_actor};
use crate::reg::Registry;
use crate::utils::NamespacedKey;
use avian3d::prelude::Collider;
use bevy::prelude::*;
use std::sync::LazyLock;

pub static KEY: LazyLock<NamespacedKey> = LazyLock::new(|| NamespacedKey::new_embers("creeper"));

#[derive(Component, Debug)]
pub struct Creeper {}

pub fn creeper(attribute_bases: &Registry<AttributeBase>) -> impl Bundle {
    (
        living_actor(&KEY, attribute_bases),
        Collider::cylinder(0.5, 1.7),
        Creeper {},
    )
}
