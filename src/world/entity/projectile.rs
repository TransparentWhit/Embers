use avian3d::prelude::RigidBody;
use bevy::prelude::*;
use super::entity;

#[derive(Component)]
struct Projectile {}

pub fn projectile() -> impl Bundle {
    (entity(), Projectile {}, RigidBody::Dynamic)
}
