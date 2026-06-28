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

#[derive(Component, Debug)]
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
    mut particles: ResMut<Assets<ParticleSystemAsset>>,
    time: Res<Time>,
) {
    for (entity, transform, mut animation_player, mut fuse) in query.iter_mut() {
        if fuse.is_added() {
            /*commands.entity(entity).insert(animate_actor(
                &payload_manager,
                &mut animation_player,
                &asset_server,
                &models,
                &MODEL_KEY,
                GltfElementId::Default,
            ).expect("Requires animation"));*/
        }
        fuse.0.tick(time.delta());
        if fuse.0.is_finished() {
            /*let mut module = Module::default();
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
                    images: vec![asset_server.load("global/textures/particles/explosion_10.png")],
                },
                *transform,
            ));*/
            commands.spawn((
                ParticleSystem3D {
                    handle: particles.add(ParticleSystemAsset::new(
                        "Explosion".into(),
                        ParticleSystemDimension::D3,
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
                            accelerations: EmitterAccelerations {
                                gravity: Vec3::new(0.0, -0.08, 0.0),
                                ..default()
                            },
                            ..default()
                        }],
                        vec![],
                        true,
                        default(),
                    )),
                },
                transform.clone(),
            ));
            commands.entity(entity).despawn();
        }
    }
}

pub fn primed_tnt(
    payload_manager: &PayloadManager,
    asset_server: &AssetServer,
    models: &Assets<Gltf>,
) -> impl Bundle {
    (
        actor(),
        PhysicsPreset::MiscActor.physics(false),
        Fuse::default(),
        AnimationPlayer::default(),
        SceneRoot(
            actor_scene(
                payload_manager,
                asset_server,
                models,
                &MODEL_KEY,
                GltfElementId::Default,
            )
            .unwrap(),
        ),
        Collider::cuboid(1.0, 1.0, 1.0),
    )
}
