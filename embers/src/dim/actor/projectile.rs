use super::actor;
use crate::dim::PhysicsPreset;
use bevy::prelude::*;

pub fn projectile() -> impl Scene {
    bsn! {
        actor()
        { PhysicsPreset::Projectile.physics(false) }
    }
}
