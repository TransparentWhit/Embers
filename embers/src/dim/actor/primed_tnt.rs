use super::super::PhysicsPreset;
use super::actor;
use crate::pld::GLOBAL_PAYLOADS;
use crate::utils::NamespacedKey;
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_hanabi::{
    EffectAsset, EffectMaterial, Module, ParticleEffect, SetPositionSphereModifier,
    SetVelocitySphereModifier, ShapeDimension, SpawnerSettings,
};
use std::sync::LazyLock;
use std::time::Duration;

pub static KEY: LazyLock<NamespacedKey> = LazyLock::new(|| NamespacedKey::new_embers("tnt"));

static MODEL_KEY: LazyLock<NamespacedKey> = LazyLock::new(|| NamespacedKey::new_embers("tnt"));

#[derive(Component, Debug)]
pub struct Fuse(Timer);

impl Default for Fuse {
    fn default() -> Self {
        Self(Timer::new(Duration::from_secs_f32(4.0), TimerMode::Once))
    }
}

pub(in crate::dim::actor) fn fuse(
    mut commands: Commands,
    mut query: Query<(Entity, &Transform, &mut AnimationPlayer, Mut<Fuse>)>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
) {
    for (entity, transform, mut animation_player, mut fuse) in query.iter_mut() {
        if fuse.is_added() {
            commands
                .entity(entity)
                .insert(GLOBAL_PAYLOADS.animate_actor(
                    &mut animation_player,
                    &asset_server,
                    &MODEL_KEY,
                    0,
                ));
        }
        fuse.0.tick(time.delta());
        if fuse.0.is_finished() {
            let mut module = Module::default();
            let init_pos = SetPositionSphereModifier {
                center: module.lit(Vec3::ZERO),
                radius: module.lit(2.),
                dimension: ShapeDimension::Surface,
            };
            let init_vel = SetVelocitySphereModifier {
                center: module.lit(Vec3::ZERO),
                speed: module.lit(6.),
            };
            commands.spawn((
                ParticleEffect::new(
                    asset_server.add(
                        EffectAsset::new(16, SpawnerSettings::once((4.).into()), module)
                            .init(init_pos)
                            .init(init_vel),
                    ),
                ),
                EffectMaterial {
                    images: vec![asset_server.load("global/images/particles/explosion_10.png")],
                },
                *transform,
            ));
            commands.entity(entity).despawn();
        }
    }
}

pub fn primed_tnt(asset_server: &AssetServer) -> impl Bundle {
    (
        actor(),
        PhysicsPreset::MiscActor.physics(),
        Fuse::default(),
        AnimationPlayer::default(),
        SceneRoot(GLOBAL_PAYLOADS.actor_scene(asset_server, &MODEL_KEY, 0)),
        Collider::cuboid(1.0, 1.0, 1.0),
    )
}
