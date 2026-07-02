use super::actor;
use crate::dim::PhysicsPreset;
use crate::pld::{GltfElementId, PayloadManager, actor_scene, animate_actor};
use crate::utils::NamespacedKey;
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_sprinkles::prelude::*;
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
    mut query: Query<(Entity, &Transform, &mut AnimationPlayer, Mut<Fuse>)>,
    payload_manager: Res<PayloadManager>,
    asset_server: Res<AssetServer>,
    models: Res<Assets<Gltf>>,
    mut particles: ResMut<Assets<ParticlesAsset>>,
    time: Res<Time>,
) {
    for (entity, transform, mut animation_player, mut fuse) in query.iter_mut() {
        fuse.0.tick(time.delta());
        if fuse.0.is_finished() {
            commands.spawn_scene(bsn! {
                Particles3d(asset_value(ParticlesAsset::new(
                    "Explosion".into(),
                    ParticlesDimension::D3,
                    default(),
                    vec![EmitterData {
                        time: EmitterTime {
                            lifetime: 2.95,
                            lifetime_randomness: 0.8426,
                            one_shot: true,
                            explosiveness: 1.0,
                            ..default()
                        },
                        draw_pass: EmitterDrawPass {
                            mesh: ParticleMesh::Quad {
                                orientation: default(),
                                size: Vec2::ONE,
                                subdivide: Vec2::ZERO,
                            },
                            material: DrawPassMaterial::Standard(StandardParticleMaterial {
                                alpha_mode: SerializableAlphaMode::Blend,
                                base_color_texture: Some(TextureRef::Asset(
                                    "global/textures/particles/explosion_10.png".to_string(),
                                )),
                                unlit: true,
                                ..default()
                            }),
                            transform_align: Some(TransformAlign::Billboard),
                            ..default()
                        },
                        emission: EmitterEmission {
                            shape: EmissionShape::Sphere { radius: 1. },
                            particles_amount: 8,
                            ..default()
                        },
                        accelerations: EmitterAccelerations {
                            gravity: Vec3::new(0.0, -0.08, 0.0),
                            ..default()
                        },
                        ..default()
                    }],
                    vec![],
                    true,
                    default(),
                )))
            });
            commands.spawn((Particles3d(particles.add()), transform.clone()));
            commands.entity(entity).despawn();
        }
    }
}

pub fn primed_tnt() -> impl Scene {
    bsn! {
        actor()
        { PhysicsPreset::MiscActor.physics(false) }
        Fuse
        WorldAssetRoot({
            actor_scene(
                &MODEL_KEY,
                GltfElementId::Default,
            )
        })
        //{ Collider::cuboid(1.0, 1.0, 1.0) }
    }
}
