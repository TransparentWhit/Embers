use super::{GameState, RootNode};
use crate::dim::actor::item_actor::item_actor_of;
use crate::dim::actor::living::AttributeBase;
use crate::dim::actor::living::dummy::dummy;
use crate::dim::actor::living::player::{Player, player};
use crate::dim::item::{sword, tnt};
use crate::dim::{PhysicsPreset, dimensional_gateway};
use crate::pld::PayloadManager;
use crate::reg::Reg;
use avian3d::prelude::*;
use bevy::camera::{ScalingMode, Viewport};
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowResized};
use std::ops::DerefMut;

#[derive(Component)]
pub struct DimensionRootNode;

#[derive(Component)]
struct Ground;

#[derive(Component, Debug)]
pub enum PlayerCamera {
    Isometric {
        distance: f32,
        height: f32,
        /// **In radians**
        angle: f32,
    },
}

fn init(
    mut commands: Commands,
    payload_manager: Res<PayloadManager>,
    asset_server: Res<AssetServer>,
    models: Res<Assets<Gltf>>,
    attribute_bases: Reg<AttributeBase>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        RootNode,
        DespawnOnExit(GameState::Dimension),
        Transform::default(),
        Node {
            width: percent(100),
            height: percent(100),
            display: Display::Flex,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        children![
            (
                DimensionRootNode,
                Node {
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
            ),
            (
                Camera::default(),
                Camera3d::default(),
                Bloom::default(),
                Projection::from(OrthographicProjection {
                    scaling_mode: ScalingMode::Fixed {
                        width: 16.,
                        height: 9.,
                    },
                    ..OrthographicProjection::default_3d()
                }),
                PlayerCamera::Isometric {
                    distance: 12.,
                    height: 8.,
                    angle: 35f32.to_radians(),
                },
            ),
            (
                DirectionalLight::default(),
                Transform::from_translation(Vec3::ONE).looking_at(Vec3::ZERO, Vec3::Y),
            ),
            (
                Mesh3d(meshes.add(Plane3d::default().mesh().size(20., 20.))),
                MeshMaterial3d(materials.add(Color::WHITE)),
                PhysicsPreset::Environment.physics(false),
                Ground,
                Collider::heightfield(vec![vec![0.0, 0.0], vec![0.0, 0.0]], Vec3::splat(20.)),
            ),
            (dimensional_gateway(&asset_server),),
            (
                Mesh3d(
                    meshes.add(
                        Cylinder {
                            radius: 0.5,
                            half_height: 0.85,
                        }
                        .mesh(),
                    ),
                ),
                MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
                player(attribute_bases.as_ref()),
                Transform::from_xyz(0.0, 1.0, 0.0),
                LinearVelocity::from(Vec3::new(0., 10., 0.)),
            ),
            (
                dummy(
                    &payload_manager,
                    &asset_server,
                    &models,
                    attribute_bases.as_ref()
                ),
                Transform::from_xyz(5.0, 0.5, 0.0)
            ),
            (
                item_actor_of(&payload_manager, &asset_server, &models, sword()),
                Transform::from_xyz(2.0, 1.0, 0.0),
            ),
            (
                item_actor_of(&payload_manager, &asset_server, &models, tnt()),
                Transform::from_xyz(2.0, 1.0, 0.0),
            ),
        ],
    ));
}

fn resize_camera(
    primary_window: Single<&Window, With<PrimaryWindow>>,
    mut player_camera: Single<&mut Camera, With<PlayerCamera>>,
) {
    let size = primary_window.physical_size();
    let physical_position: UVec2;
    let physical_size: UVec2;
    if size.x * 9 > size.y * 16 {
        physical_position = UVec2::new((size.x - (size.y * 16 / 9)) / 2, 0);
        physical_size = UVec2::new(size.y * 16 / 9, size.y);
    } else {
        physical_position = UVec2::new(0, (size.y - (size.x * 9 / 16)) / 2);
        physical_size = UVec2::new(size.x, size.x * 9 / 16);
    }
    player_camera.viewport = Some(Viewport {
        physical_position,
        physical_size,
        ..default()
    });
}

fn update_player_camera(
    player: Single<&Transform, With<Player>>,
    camera: Option<Single<(&mut Transform, &PlayerCamera), (With<PlayerCamera>, Without<Player>)>>,
) {
    if let Some(mut camera) = camera {
        let (camera_transform, config) = camera.deref_mut();
        match config {
            PlayerCamera::Isometric {
                distance,
                height,
                angle,
            } => {
                let player_pos = player.translation;
                camera_transform.translation =
                    player_pos + Vec3::new(distance * angle.cos(), *height, distance * angle.sin());
                camera_transform.look_at(player_pos, Vec3::Y);
            }
        }
    }
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Dimension), (init, resize_camera).chain())
        .add_systems(Update, resize_camera.run_if(on_message::<WindowResized>))
        .add_systems(Update, update_player_camera);
}
