use super::actor;
use crate::dim::{Explosion, PhysicsPreset};
use crate::pld::{GltfElementId, actor_scene};
use crate::utils::NamespacedKey;
use avian3d::prelude::*;
use bevy::prelude::*;
use std::sync::LazyLock;
use std::time::Duration;

pub static KEY: LazyLock<NamespacedKey> = LazyLock::new(|| NamespacedKey::new_embers("tnt"));

static MODEL_KEY: LazyLock<NamespacedKey> = LazyLock::new(|| NamespacedKey::new_embers("tnt"));

#[derive(Component, Clone, Debug)]
pub struct Fuse(Timer);

impl Default for Fuse {
    fn default() -> Self {
        Self(Timer::new(Duration::from_secs_f32(4.0), TimerMode::Once))
    }
}

pub(super) fn fuse(
    mut commands: Commands,
    mut query: Query<(Entity, &GlobalTransform, Mut<Fuse>)>,
    time: Res<Time>,
) {
    for (entity, transform, mut fuse) in query.iter_mut() {
        fuse.0.tick(time.delta());
        if fuse.0.is_finished() {
            commands.trigger(Explosion {
                power: 4.,
                position: transform.translation(),
            });
            commands.entity(entity).despawn();
        }
    }
}

pub fn primed_tnt() -> impl Scene {
    bsn! {
        actor()
        { PhysicsPreset::MiscActor.physics(false) }
        Collider::cuboid(1.0, 1.0, 1.0)
        Fuse
        actor_scene(&MODEL_KEY, GltfElementId::Default)
    }
}
