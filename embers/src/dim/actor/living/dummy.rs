use super::living_actor;
use crate::pld::{GltfElementId, actor_scene};
use crate::utils::NamespacedKey;
use avian3d::prelude::*;
use bevy::prelude::*;
use std::sync::LazyLock;

pub static KEY: LazyLock<NamespacedKey> = LazyLock::new(|| NamespacedKey::new_embers("dummy"));

pub fn dummy() -> impl Scene {
    bsn! {
        living_actor(&KEY, false)
        Collider::cuboid(1., 3., 1.)
        actor_scene(&KEY, GltfElementId::Default)
    }
}
