use super::{GameState, RootNode};
use crate::dim::actor::living::player::Player;
use bevy::camera::{ScalingMode, Viewport};
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowResized};

#[derive(Clone, Component, Default)]
pub struct DimensionRootNode;

#[derive(Clone, Component, Debug)]
pub enum PlayerCamera {
    Isometric {
        distance: f32,
        height: f32,
        angle_rad: f32,
    },
}

impl Default for PlayerCamera {
    fn default() -> Self {
        Self::Isometric {
            distance: 12.,
            height: 8.,
            angle_rad: 35f32.to_radians(),
        }
    }
}

fn init(mut commands: Commands) {
    commands.spawn_scene(bsn! {
        RootNode
        DespawnOnExit<GameState>(GameState::Dimension)
        Transform
        Node {
            width: percent(100),
            height: percent(100),
            display: Display::Flex,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
        }
        Children [
            (
                DimensionRootNode
                Node {
                    width: percent(100),
                    height: percent(100),
                }
            ),
            (
                Camera
                Camera3d
                PlayerCamera
                Projection::from({OrthographicProjection {
                    scaling_mode: ScalingMode::Fixed {
                        width: 16.,
                        height: 9.,
                    },
                    ..OrthographicProjection::default_3d()
                }})
                Bloom
            ),
        ]
    });
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
    mut camera: Single<(&mut Transform, &PlayerCamera), (With<PlayerCamera>, Without<Player>)>,
) {
    let (camera_transform, config) = &mut *camera;
    match config {
        PlayerCamera::Isometric {
            distance,
            height,
            angle_rad,
        } => {
            let player_pos = player.translation;
            camera_transform.translation = player_pos
                + Vec3::new(
                    distance * angle_rad.cos(),
                    *height,
                    distance * angle_rad.sin(),
                );
            camera_transform.look_at(player_pos, Vec3::Y);
        }
    }
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Dimension), (init, resize_camera).chain())
        .add_systems(Update, resize_camera.run_if(on_message::<WindowResized>))
        .add_systems(Update, update_player_camera);
}
