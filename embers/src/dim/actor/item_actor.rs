use super::super::PhysicsPreset;
use crate::dim::Interactable;
use crate::dim::actor::actor;
use crate::pld::GLOBAL_PAYLOADS;
use crate::utils::NamespacedKey;
use avian3d::prelude::*;
use bevy::ecs::relationship::Relationship;
use bevy::ecs::spawn::SpawnableList;
use bevy::prelude::*;
use bevy::ptr::{MovingPtr, deconstruct_moving_ptr};
use std::sync::LazyLock;

pub static KEY: LazyLock<NamespacedKey> = LazyLock::new(|| NamespacedKey::new_embers("item"));

pub static INTERACTION_PICKUP: LazyLock<NamespacedKey> =
    LazyLock::new(|| NamespacedKey::new_embers("item_actor/pickup"));

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

#[derive(Component, Debug)]
pub struct ItemActor(pub Entity);

#[inline]
fn item_actor(asset_server: &AssetServer) -> impl Bundle {
    (
        actor(),
        PhysicsPreset::Phantom.physics(true),
        Collider::cuboid(0.25, 0.25, 0.25),
        SceneRoot(GLOBAL_PAYLOADS.default_model(asset_server)),
        Interactable {
            distance_factor: 1.,
            initial_click: Some(INTERACTION_PICKUP.clone()),
            initial_double_click: None,
        },
    )
}

pub fn item_actor_for(asset_server: &AssetServer, item: Entity) -> impl Bundle {
    (item_actor(asset_server), ItemActor(item))
}

pub fn item_actor_of(asset_server: &AssetServer, item: impl Bundle) -> impl Bundle {
    (item_actor(asset_server), Children::spawn(SpawnItem(item)))
}
