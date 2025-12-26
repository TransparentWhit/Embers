use crate::dim::LOBBY;
use crate::dim::actor::living::{AttributeBase, living_actor};
use crate::reg::RegistryAccess;
use crate::utils::NamespacedKey;
use avian3d::prelude::*;
use bevy::prelude::*;
use std::sync::LazyLock;

pub static KEY: LazyLock<NamespacedKey> = LazyLock::new(|| NamespacedKey::new_embers("dummy"));

static MODEL_KEY: LazyLock<NamespacedKey> = LazyLock::new(|| NamespacedKey::new_embers("dummy"));

pub fn dummy(
    asset_server: &AssetServer,
    attribute_bases: impl RegistryAccess<Item = AttributeBase>,
) -> impl Bundle {
    (
        living_actor(&KEY, attribute_bases),
        Collider::cuboid(1., 3., 1.),
        SceneRoot(LOBBY.assets().actor_scene(asset_server, &MODEL_KEY, 0)),
    )
}
