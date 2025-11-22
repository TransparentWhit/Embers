use super::{Attributes, living_entity};
use crate::ui::world::PlayerCamera;
use crate::utils::NamespacedKey;
use crate::world::entity::living::attributes::embers;
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_tnua::prelude::*;
use std::collections::HashMap;
use std::ops::DerefMut;
use std::sync::LazyLock;

pub(in crate::world) fn process_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut player: Single<(&Attributes, &mut TnuaController), With<Player>>,
    player_camera: Single<(&Camera, &PlayerCamera), With<PlayerCamera>>,
) {
    let (attributes, controller) = player.deref_mut();
    if mouse.pressed(MouseButton::Left)
        && let Some(physical_cursor_position) = window.physical_cursor_position()
    {
        let (camera, camera_config) = *player_camera;
        let forward = match camera_config {
            PlayerCamera::Isometric {
                distance,
                height,
                angle,
            } => {
                let viewport = camera.viewport.clone().unwrap_or_default();
                let normalized = (physical_cursor_position - viewport.physical_position.as_vec2())
                    / viewport.physical_size.as_vec2();
                let forward = Vec3::new(-distance * angle.cos(), -height, -distance * angle.sin())
                    .normalize();
                let right = forward.cross(Vec3::Y).normalize();
                Dir3::new(
                    ((right * (normalized.x * 2. - 1.))
                        + (Vec3::Y.cross(right) * (1. - normalized.y * 2.)))
                        .with_y(0.),
                )
                .ok()
            }
        };
        controller.basis(TnuaBuiltinWalk {
            float_height: FLOAT_HEIGHT,
            desired_velocity: forward
                .map(|direction| {
                    direction.as_vec3() * attributes.0[&embers::MOVEMENT_SPEED].value()
                })
                .unwrap_or_default(),
            desired_forward: forward,
            ..default()
        });
    }
}

static ATTRIBUTES: LazyLock<HashMap<NamespacedKey, f32>> = LazyLock::new(|| {
    HashMap::from([
        (embers::MAX_HEALTH.clone(), 20.),
        (embers::MOVEMENT_SPEED.clone(), 2.),
    ])
});
static HITBOX: LazyLock<Collider> = LazyLock::new(|| Collider::cylinder(0.5, 1.7));
const FLOAT_HEIGHT: f32 = 0.85;

#[derive(Component)]
pub struct Player {
    pub flops: i32,
    pub hashes: i32,
    pub time_crystals: i32,
}

pub fn player() -> impl Bundle {
    (
        living_entity(&ATTRIBUTES),
        HITBOX.clone(),
        Player {
            flops: 0,
            hashes: 0,
            time_crystals: 0,
        },
    )
}
