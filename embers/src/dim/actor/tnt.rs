use super::actor;
use crate::utils::assets::GLOBAL_ASSETS;
use crate::utils::NamespacedKey;
use avian3d::prelude::*;
use bevy::prelude::*;
use std::sync::LazyLock;
use std::time::Duration;

static MODEL_KEY: LazyLock<NamespacedKey> = LazyLock::new(|| NamespacedKey::new_embers("tnt"));

#[derive(Component, Debug)]
pub struct Fuse(Timer);
impl Default for Fuse {
    fn default() -> Self {
        Self(Timer::new(Duration::from_secs_f32(4.0), TimerMode::Once))
    }
}

const FLASH_INTERVAL: f32 = 0.5;
const TWO_FLASH_INTERVALS: f32 = FLASH_INTERVAL * 2.0;

pub(in crate::dim::actor) fn fuse(
    mut commands: Commands,
    mut query: Query<(Entity, &mut AnimationPlayer, Mut<Fuse>)>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
) {
    for (entity, mut animation_player, mut fuse) in query.iter_mut() {
        if fuse.is_added() {
            commands.entity(entity).insert(GLOBAL_ASSETS.animate_actor(
                &mut animation_player,
                &asset_server,
                &MODEL_KEY,
                0,
            ));
        }
        fuse.0.tick(time.delta());
        if fuse.0.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

pub fn tnt(asset_server: &AssetServer) -> impl Bundle {
    (
        actor(),
        Fuse::default(),
        AnimationPlayer::default(),
        SceneRoot(GLOBAL_ASSETS.actor_scene(asset_server, &MODEL_KEY, 0)),
        Collider::cuboid(1.0, 1.0, 1.0),
        RigidBody::Dynamic,
    )
}
