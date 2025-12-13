pub mod entity;
pub mod item;

use crate::GameState;
use crate::utils::assets::AssetScope;
use crate::utils::{Keyed, Namespaced, NamespacedKey};
use bevy::prelude::*;
use std::sync::LazyLock;

/// Time of the day, within [0, 1).
#[derive(Component)]
pub struct Time(pub f32);
impl Default for Time {
    fn default() -> Self {
        Self(0.25)
    }
}

pub struct World {
    key: NamespacedKey,
    assets: AssetScope,
}
impl Keyed for World {
    fn key(&self) -> &NamespacedKey {
        &self.key
    }
}
impl World {
    pub fn new(key: NamespacedKey) -> Self {
        Self {
            assets: AssetScope::new(format!("world/{}/{}", key.namespace(), key.key())),
            key,
        }
    }
    pub fn assets(&self) -> &AssetScope {
        &self.assets
    }
}

pub static LOBBY: LazyLock<World> =
    LazyLock::new(|| World::new(NamespacedKey::new_embers("lobby")));

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(entity::plugin);
    app.add_plugins(item::plugin);
    app.add_systems(
        Update,
        entity::living::player::process_input.run_if(in_state(GameState::World)),
    );
}
