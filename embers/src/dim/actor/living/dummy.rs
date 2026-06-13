use crate::dim::LOBBY;
use crate::dim::actor::ACTOR_NAMESPACE;
use crate::dim::actor::living::{AttributeBase, living_actor};
use crate::reg::Registry;
use crate::utils::NamespacedKey;
use avian3d::prelude::*;
use bevy::prelude::*;
use std::sync::LazyLock;
use uuid::Uuid;

pub static KEY: LazyLock<NamespacedKey> = LazyLock::new(|| NamespacedKey::new_embers("dummy"));

pub static UUID: LazyLock<Uuid> =
    LazyLock::new(|| Uuid::new_v5(&ACTOR_NAMESPACE, KEY.to_string().as_bytes()));

static MODEL_KEY: LazyLock<NamespacedKey> = LazyLock::new(|| NamespacedKey::new_embers("dummy"));

pub fn dummy(asset_server: &AssetServer, attribute_bases: &Registry<AttributeBase>) -> impl Bundle {
    (
        living_actor(&KEY, &UUID, attribute_bases, false),
        Collider::cuboid(1., 3., 1.),
        SceneRoot(LOBBY.payloads().actor_scene(asset_server, &MODEL_KEY, 0)),
    )
}
