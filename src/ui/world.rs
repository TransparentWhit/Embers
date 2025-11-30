use crate::GameState;
use crate::ui::scalable;
use crate::utils::assets::GLOBAL_ASSETS;
use crate::world::entity::living::player::{HOTBAR_SLOTS, Player, player};
use crate::world::entity::tnt::tnt;
use avian3d::prelude::*;
use bevy::camera::{ScalingMode, Viewport};
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowResized};
use std::ops::DerefMut;

#[derive(States, Clone, Copy, Default, Eq, PartialEq, Debug, Hash)]
enum WorldState {
    Main,
    Options,
    #[default]
    Disabled,
}

#[derive(Component)]
struct Ground;

#[derive(Component)]
pub enum PlayerCamera {
    Isometric {
        distance: f32,
        height: f32,
        /// **In radians**
        angle: f32,
    },
}

#[derive(Component)]
struct HotbarSlot(u8);

#[derive(Component)]
struct MainhandSlot;

fn init(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        DespawnOnExit(GameState::World),
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
                Node {
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
                children![(
                    scalable(|scale| Node {
                        left: percent(50),
                        bottom: px(scale * 7),
                        margin: UiRect::left(px(scale * -185 / 2)),
                        position_type: PositionType::Absolute,
                        width: px(scale * 185),
                        height: px(scale * 18),
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    }),
                    children![
                        (
                            scalable(|scale| Node {
                                width: px(scale * 122),
                                height: px(scale * 22),
                                margin: UiRect::horizontal(px(scale * 3)),
                                ..default()
                            }),
                            GLOBAL_ASSETS.image_node(&asset_server, "hotbar"),
                            Children::spawn({
                                let mut hotbar_slots = Vec::with_capacity(HOTBAR_SLOTS as usize);
                                for i in 0..HOTBAR_SLOTS {
                                    hotbar_slots.push((
                                        scalable(|scale| Node {
                                            display: Display::None,
                                            width: px(scale * 16),
                                            height: px(scale * 16),
                                            margin: UiRect::horizontal(px(scale * 2)),
                                            ..default()
                                        }),
                                        ImageNode::default(),
                                        HotbarSlot(i),
                                    ));
                                }
                                (
                                    hotbar_slots,
                                    Spawn((
                                        scalable(|scale| Node {
                                            display: Display::None,
                                            width: px(scale * 24),
                                            height: px(scale * 23),
                                            ..default()
                                        }),
                                        GLOBAL_ASSETS.image_node(&asset_server, "hotbar_selection"),
                                        children![(ImageNode::default(), MainhandSlot,),],
                                    )),
                                )
                            })
                        ),
                        (
                            scalable(|scale| Node {
                                width: px(scale * 22),
                                height: px(scale * 22),
                                margin: UiRect::horizontal(px(scale * 3)),
                                ..default()
                            }),
                            GLOBAL_ASSETS.image_node(&asset_server, "main_hand"),
                            children![(ImageNode::default(), MainhandSlot,),]
                        ),
                    ],
                ),]
            ),
            (
                Camera::default(),
                Camera3d::default(),
                Bloom::default(),
                Projection::from(OrthographicProjection {
                    scaling_mode: ScalingMode::Fixed {
                        width: 16f32,
                        height: 9f32,
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
                Ground,
                RigidBody::Static,
                Collider::heightfield(vec![vec![0.0, 0.0], vec![0.0, 0.0]], Vec3::splat(20.)),
            ),
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
                player(),
                Transform::from_xyz(0.0, 1.0, 0.0),
                LinearVelocity::from(Vec3::new(0., 10., 0.)),
            ),
            (tnt(&asset_server), Transform::from_xyz(0.0, 0.5, 0.0)),
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
    app.add_systems(OnEnter(GameState::World), (init, resize_camera).chain());
    app.add_systems(Update, resize_camera.run_if(on_message::<WindowResized>));
    app.add_systems(Update, update_player_camera);
}
