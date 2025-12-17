use crate::dim::actor::living::living_actor;
use crate::dim::LOBBY;
use crate::utils::NamespacedKey;
use avian3d::prelude::*;
use bevy::prelude::*;
use std::collections::HashMap;
use std::sync::LazyLock;

static ATTRIBUTES: LazyLock<HashMap<NamespacedKey, f32>> =
    LazyLock::new(|| HashMap::from([(NamespacedKey::new("embers", "max_health"), f32::INFINITY)]));
static MODEL_KEY: LazyLock<NamespacedKey> = LazyLock::new(|| NamespacedKey::new_embers("dummy"));

pub fn dummy(asset_server: &AssetServer) -> impl Bundle {
    (
        living_actor(&ATTRIBUTES),
        Collider::cuboid(1., 3., 1.),
        SceneRoot(LOBBY.assets().actor_scene(asset_server, &MODEL_KEY, 0)),
    )
}
