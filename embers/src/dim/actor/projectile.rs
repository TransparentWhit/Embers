use super::actor;
use crate::dim::PhysicsPreset;
use bevy::prelude::*;

pub fn projectile() -> impl Bundle {
    (actor(), PhysicsPreset::Projectile.physics(false))
}
