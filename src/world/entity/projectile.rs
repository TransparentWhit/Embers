use super::entity;
use avian3d::prelude::RigidBody;
use bevy::prelude::*;

pub fn projectile() -> impl Bundle {
    (entity(), RigidBody::Dynamic)
}
