use super::super::PhysicsPreset;
use super::actor;
use bevy::prelude::*;

pub fn projectile() -> impl Bundle {
    (actor(), PhysicsPreset::Projectile.physics(false))
}
