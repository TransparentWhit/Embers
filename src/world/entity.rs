pub mod projectile;
pub mod living;

use bevy::prelude::*;

#[derive(Component)]
pub struct Entity;

pub fn entity() -> impl Bundle {
    Entity
}
