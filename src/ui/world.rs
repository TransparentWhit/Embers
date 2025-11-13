use crate::{GameState};
use avian3d::prelude::*;
use bevy::prelude::*;
use crate::world::entity::player::Player;

#[derive(Component)]
struct Ground;

#[derive(Component)]
struct IsometricCamera {
    pub distance: f32,
    pub height: f32,
    /// **In radians**
    pub angle: f32,
    pub follow_speed: f32,
}

#[derive(Component)]
struct PlayerCamera;

fn init(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        DespawnOnExit(GameState::World),
        Transform::default(),
        Node {
            width: Val::Percent(100f32),
            height: Val::Percent(100f32),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        children![
            (
                Camera3d::default(),
                Transform::from_xyz(15.0, 5.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
                IsometricCamera {
                    distance: 12.0,
                    height: 8.0,
                    angle: 35.0f32.to_radians(),
                    follow_speed: 0.05,
                },
                PlayerCamera,
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
                Mesh3d(meshes.add(Cylinder { radius: 0.5, half_height: 0.85 }.mesh())),
                MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
                Player,
                RigidBody::Dynamic,
                Collider::cylinder(0.5, 1.7),
                Transform::from_xyz(0.0, 1.0, 0.0),
                LinearVelocity::from(Vec3::new(0., 10., 0.)),
            ),
        ],
    ));
}

fn update_player_camera(
    player_query: Query<&Transform, With<Player>>,
    mut camera_query: Query<(&mut Transform, &IsometricCamera), (With<PlayerCamera>, With<IsometricCamera>, Without<Player>)>,
) {
    if let Ok(player_transform) = player_query.single() {
        if let Ok((mut camera_transform, config)) = camera_query.single_mut() {
            let player_pos = player_transform.translation;
            let camera_offset = Vec3::new(
                config.distance * config.angle.cos(),
                config.height,
                config.distance * config.angle.sin(),
            );
            let target_position = player_pos + camera_offset;
            camera_transform.translation = camera_transform.translation.lerp(
                target_position,
                config.follow_speed
            );
            camera_transform.look_at(player_pos, Vec3::Y);
        }
    }
}

pub fn world_plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::World), init);
    app.add_systems(Update, update_player_camera);
}
