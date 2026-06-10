use crate::dim::LOBBY;
use crate::dim::actor::living::{AttributeBase, living_actor};
use crate::reg::Registry;
use crate::utils::NamespacedKey;
use avian3d::prelude::*;
use bevy::prelude::*;
use std::sync::LazyLock;

pub static KEY: LazyLock<NamespacedKey> = LazyLock::new(|| NamespacedKey::new_embers("dummy"));

static MODEL_KEY: LazyLock<NamespacedKey> = LazyLock::new(|| NamespacedKey::new_embers("dummy"));

const FLOAT_HEIGHT: f32 = 1.5;

pub fn dummy(asset_server: &AssetServer, attribute_bases: &Registry<AttributeBase>) -> impl Bundle {
    (
        living_actor(&KEY, attribute_bases, FLOAT_HEIGHT, false),
        Collider::cuboid(1., 3., 1.),
        SceneRoot(LOBBY.payloads().actor_scene(asset_server, &MODEL_KEY, 0)),
    )
}
