use std::collections::HashMap;
use std::sync::LazyLock;
use avian3d::prelude::Collider;
use bevy::prelude::*;
use crate::utils::NamespacedKey;
use super::living_entity;

fn process_input(keys: Res<ButtonInput<KeyCode>>, mouse: Res<ButtonInput<MouseButton>>) {
    mouse.pressed(MouseButton::Left);
}

static ATTRIBUTES: LazyLock<HashMap<NamespacedKey, f32>> = LazyLock::new(|| {HashMap::from([
    (NamespacedKey::new("embers", "max_health"), 20f32),
])});
static HITBOX: LazyLock<Collider> = LazyLock::new(|| Collider::cylinder(0.5, 1.7));

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
