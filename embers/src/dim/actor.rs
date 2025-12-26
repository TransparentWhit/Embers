pub mod item_actor;
pub mod living;
pub mod primed_tnt;
pub mod projectile;

use crate::GameState;
use bevy::prelude::*;

#[derive(Component, Debug)]
pub struct Actor;

pub fn actor() -> impl Bundle {
    Actor
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (primed_tnt::fuse).run_if(in_state(GameState::Dimension)),
    )
    .add_plugins(living::plugin);
}
