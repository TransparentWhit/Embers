use super::actor;
use avian3d::prelude::RigidBody;
use bevy::prelude::*;

pub fn projectile() -> impl Bundle {
    (actor(), RigidBody::Dynamic)
}
