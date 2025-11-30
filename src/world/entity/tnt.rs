use super::entity;
use crate::utils::NamespacedKey;
use crate::utils::assets::GLOBAL_ASSETS;
use avian3d::prelude::*;
use bevy::prelude::*;
use std::sync::LazyLock;
use std::time::Duration;

static MODEL_KEY: LazyLock<NamespacedKey> = LazyLock::new(|| NamespacedKey::new_embers("tnt"));
static HITBOX: LazyLock<Collider> = LazyLock::new(|| Collider::cuboid(1.0, 1.0, 1.0));

#[derive(Component)]
pub struct Fuse(Timer);
impl Default for Fuse {
    fn default() -> Self {
        Self(Timer::new(Duration::from_secs_f32(4.0), TimerMode::Once))
    }
}

const FLASH_INTERVAL: f32 = 0.5;
const TWO_FLASH_INTERVALS: f32 = FLASH_INTERVAL * 2.0;

pub(in crate::world::entity) fn fuse(
    mut commands: Commands,
    mut query: Query<(Entity, &mut AnimationPlayer, Mut<Fuse>)>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
) {
    for (entity, mut animation_player, mut fuse) in query.iter_mut() {
        if fuse.is_added() {
            commands.entity(entity).insert(GLOBAL_ASSETS.animate_entity(
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
        entity(),
        Fuse::default(),
        AnimationPlayer::default(),
        SceneRoot(GLOBAL_ASSETS.entity_scene(asset_server, &MODEL_KEY, 0)),
        HITBOX.clone(),
        RigidBody::Dynamic,
    )
}
