pub mod projectile;
pub mod living;
pub mod tnt;

use bevy::prelude::*;

#[derive(Component)]
pub struct Entity;

pub fn entity() -> impl Bundle {
    Entity
}
