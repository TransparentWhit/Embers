use super::living_actor;
use crate::utils::NamespacedKey;
use avian3d::prelude::*;
use bevy::prelude::*;
use std::sync::LazyLock;

pub static KEY: LazyLock<NamespacedKey> = LazyLock::new(|| NamespacedKey::new_embers("creeper"));

#[derive(Clone, Component, Default)]
pub struct Creeper;

pub fn creeper() -> impl Scene {
    bsn! {
        living_actor(&KEY, false)
        Collider::cylinder(0.5, 1.7)
        Creeper
    }
}
