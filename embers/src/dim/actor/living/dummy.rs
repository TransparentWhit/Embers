use super::{AttributeBase, living_actor};
use crate::dim::actor::MOVEMENT_CONFIG_NAMESPACE;
use crate::pld::{GltfElementId, actor_scene};
use crate::reg::Registry;
use crate::utils::NamespacedKey;
use avian3d::prelude::*;
use bevy::prelude::*;
use std::sync::LazyLock;
use uuid::Uuid;

pub static KEY: LazyLock<NamespacedKey> = LazyLock::new(|| NamespacedKey::new_embers("dummy"));

pub static UUID: LazyLock<Uuid> =
    LazyLock::new(|| Uuid::new_v5(&MOVEMENT_CONFIG_NAMESPACE, KEY.to_string().as_bytes()));

static MODEL_KEY: LazyLock<NamespacedKey> = LazyLock::new(|| NamespacedKey::new_embers("dummy"));

pub fn dummy(attribute_bases: &Registry<AttributeBase>) -> impl Bundle {
    (
        living_actor(&KEY, &UUID, attribute_bases, false),
        Collider::cuboid(1., 3., 1.),
        actor_scene(&MODEL_KEY, GltfElementId::Default),
    )
}
