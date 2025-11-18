use super::entity;
use crate::utils::NamespacedKey;
use crate::utils::assets::GLOBAL_ASSETS;
use avian3d::prelude::*;
use bevy::prelude::*;
use std::sync::LazyLock;

static MODEL_KEY: LazyLock<NamespacedKey> = LazyLock::new(|| NamespacedKey::new_embers("tnt"));
static HITBOX: LazyLock<Collider> = LazyLock::new(|| Collider::cuboid(1.0, 1.0, 1.0));

#[derive(Component)]
pub struct Fuse(f32);
impl Default for Fuse {
    fn default() -> Self {
        Self(4.0)
    }
}

const FLASH_INTERVAL: f32 = 0.5;
const TWO_FLASH_INTERVALS: f32 = FLASH_INTERVAL * 2.0;

pub(in crate::world::entity) fn fuse(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Fuse)>,
    time: Res<Time>,
) {
    for (entity, mut fuse) in query.iter_mut() {
        fuse.0 -= time.delta_secs();
        if fuse.0 % TWO_FLASH_INTERVALS >= FLASH_INTERVAL {
            //commands.entity(entity).log_components();
        }
        if fuse.0.is_sign_negative() {
            commands.entity(entity).despawn();
        }
    }
}

pub fn tnt(asset_server: &AssetServer) -> impl Bundle {
    (
        entity(),
        Fuse::default(),
        SceneRoot(GLOBAL_ASSETS.entity_model(asset_server, &MODEL_KEY)),
        HITBOX.clone(),
        RigidBody::Dynamic,
    )
}
