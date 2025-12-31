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

#[derive(PhysicsLayer, Default, Copy, Clone)]
enum CollisionLayer {
    LivingActor,
    MiscActor,
    #[default]
    Phantom,
    Projectile,
    Environment,
}

impl From<CollisionLayer> for CollisionLayers {
    fn from(value: CollisionLayer) -> CollisionLayers {
        CollisionLayers::new(
            value,
            match value {
                CollisionLayer::Phantom => [
                    CollisionLayer::LivingActor,
                    CollisionLayer::MiscActor,
                    CollisionLayer::Projectile,
                    CollisionLayer::Environment,
                ]
                .into(),
                CollisionLayer::Environment => [
                    CollisionLayer::LivingActor,
                    CollisionLayer::MiscActor,
                    CollisionLayer::Phantom,
                    CollisionLayer::Projectile,
                ]
                .into(),
                _ => LayerMask::ALL,
            },
        )
    }
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum PhysicsPreset {
    LivingActor,
    MiscActor,
    Phantom,
    Projectile,
    Environment,
}

impl PhysicsPreset {
    pub fn physics(&self) -> Physics {
        (
            match self {
                Self::LivingActor => CollisionLayer::LivingActor,
                Self::MiscActor => CollisionLayer::MiscActor,
                Self::Phantom => CollisionLayer::Phantom,
                Self::Projectile => CollisionLayer::Projectile,
                Self::Environment => CollisionLayer::Environment,
            }
            .into(),
            Dominance(match self {
                Self::LivingActor => 3,
                Self::MiscActor => 2,
                Self::Phantom => 0,
                Self::Projectile => 1,
                Self::Environment => 4,
            }),
            match self {
                Self::LivingActor => LOCK_XZ_ROTATION,
                _ => FREE,
            },
            match self {
                Self::Environment => RigidBody::Static,
                _ => RigidBody::Dynamic,
            },
        )
    }
}

impl From<PhysicsPreset> for Physics {
    #[inline]
    fn from(value: PhysicsPreset) -> Physics {
        value.physics()
    }
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
