pub mod item_actor;
pub mod living;
pub mod primed_tnt;
pub mod projectile;

use crate::GameState;
use bevy::prelude::*;
use uuid::{Uuid, uuid};

pub static ACTOR_NAMESPACE: Uuid = uuid!("9e037d1a-048d-4784-8ec1-0655421951b1");

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
