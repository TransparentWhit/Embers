pub mod item_actor;
pub mod living;
pub mod projectile;
pub mod tnt;

use crate::GameState;
use bevy::prelude::*;

#[derive(Component, Debug)]
pub struct Actor;

pub fn actor() -> impl Bundle {
    Actor
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(Update, (tnt::fuse).run_if(in_state(GameState::Dimension)));
}
