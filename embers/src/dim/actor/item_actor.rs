use crate::dim::actor::actor;
use crate::dim::{Interactable, PhysicsPreset};
use crate::pld::default_scene;
use crate::utils::{NamespacedKey, Void};
use avian3d::prelude::*;
use bevy::ecs::relationship::Relationship;
use bevy::ecs::spawn::SpawnableList;
use bevy::prelude::*;
use bevy::ptr::{MovingPtr, deconstruct_moving_ptr};
use std::sync::LazyLock;
use thiserror::Error;

pub static KEY: LazyLock<NamespacedKey> = LazyLock::new(|| NamespacedKey::new_embers("item"));

pub static INTERACTION_PICKUP: LazyLock<NamespacedKey> =
    LazyLock::new(|| NamespacedKey::new_embers("item_actor/pickup"));

// TODO rework items
// TODO Apply item component prototypes when item is spawned

#[derive(Component)]
struct SpawnItem<C: Bundle>(C);

impl<R: Relationship, C: Bundle> SpawnableList<R> for SpawnItem<C> {
    fn spawn(this: MovingPtr<'_, Self>, world: &mut World, parent: Entity) {
        deconstruct_moving_ptr!({
            let SpawnItem { 0: item_bundle } = this;
        });
        let item_id = world.spawn((R::from(parent), item_bundle.read())).id();
        if let Ok(mut item_actor) = world.get_entity_mut(parent) {
            item_actor.insert(ItemActor(item_id));
        }
    }
    fn size_hint(&self) -> usize {
        1
    }
}

#[derive(Clone, Component, Copy, Debug)]
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

#[derive(Debug, Error)]
#[error("")]
struct TmpError;

pub fn item_actor_of(item: impl Bundle + Clone) -> impl Scene {
    bsn! {
        item_actor()
        template(move |context| {
            context.entity.insert(Children::spawn(SpawnItem(item.clone())));
            Err::<Void, BevyError>(BevyError::ignore(TmpError))
        })
    }
}
