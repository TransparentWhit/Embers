use super::actor;
use crate::dim::{Interactable, PhysicsPreset};
use crate::pld::default_scene;
use crate::utils::NamespacedKey;
use avian3d::prelude::*;
use bevy::prelude::*;
use std::sync::LazyLock;

pub static KEY: LazyLock<NamespacedKey> = LazyLock::new(|| NamespacedKey::new_embers("item"));

pub static INTERACTION_PICKUP: LazyLock<NamespacedKey> =
    LazyLock::new(|| NamespacedKey::new_embers("item_actor/pickup"));

#[derive(Clone, Component, Copy, Debug, FromTemplate)]
pub struct ItemActor(pub Entity);

impl Default for ItemActor {
    fn default() -> Self {
        Self(Entity::PLACEHOLDER)
    }
}

#[inline]
fn item_actor() -> impl Scene {
    bsn! {
        actor()
        { PhysicsPreset::Phantom.physics(true) }
        Collider::cuboid(0.25, 0.25, 0.25)
        default_scene()
        Interactable {
            distance_factor: 1.,
            initial_click: { Some(INTERACTION_PICKUP.clone()) },
            initial_double_click: None,
        }
    }
}

pub fn item_actor_for(item: Entity) -> impl Scene {
    bsn! {
        item_actor()
        ItemActor(item)
    }
}

pub fn item_actor_of(item: impl Scene) -> impl Scene {
    bsn! {
        item_actor()
        ItemActor(#ItemStack)
        Children [
            (
                #ItemStack
                item
            ),
        ]
    }
}
