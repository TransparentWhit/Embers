pub mod living;
pub mod projectile;
pub mod tnt;

use crate::GameState;
use bevy::prelude::*;

#[derive(Component)]
pub struct Entity;

pub fn entity() -> impl Bundle {
    Entity
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Update, (tnt::fuse).run_if(in_state(GameState::World)));
}
