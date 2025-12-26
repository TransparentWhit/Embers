pub mod actor;
pub mod item;

use crate::dim::actor::living::player;
use crate::pld::PayloadScope;
use crate::utils::{Keyed, Namespaced, NamespacedKey};
use avian3d::prelude::PhysicsLayer;
use avian3d::prelude::*;
use bevy::prelude::*;
use std::sync::LazyLock;

type Physics = (CollisionLayers, Dominance, LockedAxes, RigidBody);

const FREE: LockedAxes = LockedAxes::new();
const LOCK_XZ_ROTATION: LockedAxes = LockedAxes::new().lock_rotation_x().lock_rotation_z();

#[derive(PhysicsLayer, Debug, Default, Copy, Clone, Hash, Eq, PartialEq)]
pub enum CollisionLayer {
    LivingActor,
    MiscellaneousActor,
    PhantomActor,
    Projectile,
    #[default]
    Environment,
}

fn physics_living() -> Physics {
    (
        CollisionLayers::new(
            CollisionLayer::LivingActor,
            [
                CollisionLayer::LivingActor,
                CollisionLayer::MiscellaneousActor,
                CollisionLayer::Environment,
            ],
        ),
        Dominance(2),
        LOCK_XZ_ROTATION,
        RigidBody::Dynamic,
    )
}

fn physics_miscellaneous() -> Physics {
    (
        CollisionLayers::new(
            CollisionLayer::MiscellaneousActor,
            [
                CollisionLayer::LivingActor,
                CollisionLayer::MiscellaneousActor,
                CollisionLayer::Projectile,
                CollisionLayer::Environment,
            ],
        ),
        Dominance(1),
        LOCK_XZ_ROTATION,
        RigidBody::Dynamic,
    )
}

/// Time of the day, within [0, 1).
#[derive(Component)]
pub struct Time(pub f32);

impl Default for Time {
    fn default() -> Self {
        Self(0.25)
    }
}

pub struct Dimension {
    key: NamespacedKey,
    assets: PayloadScope<'static>,
}

impl Keyed for Dimension {
    fn key(&self) -> &NamespacedKey {
        &self.key
    }
}

impl Dimension {
    pub fn new(key: NamespacedKey) -> Self {
        Self {
            assets: PayloadScope::new(format!("dim/{}/{}", key.namespace(), key.key())),
            key,
        }
    }
    pub fn assets(&self) -> &PayloadScope<'_> {
        &self.assets
    }
}

pub static LOBBY: LazyLock<Dimension> =
    LazyLock::new(|| Dimension::new(NamespacedKey::new_embers("lobby")));

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(actor::plugin);
    app.add_plugins(item::plugin);
    app.add_plugins(player::plugin);
}
