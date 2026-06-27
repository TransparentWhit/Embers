use crate::dim::actor::actor;
use crate::dim::{Interactable, PhysicsPreset};
use crate::pld::{PayloadManager, default_model};
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
fn item_actor(
    payload_manager: &PayloadManager,
    asset_server: &AssetServer,
    scenes: &Assets<Scene>,
) -> impl Bundle {
    (
        actor(),
        PhysicsPreset::Phantom.physics(true),
        Collider::cuboid(0.25, 0.25, 0.25),
        SceneRoot(default_model(payload_manager, asset_server, scenes)),
        Interactable {
            distance_factor: 1.,
            initial_click: Some(INTERACTION_PICKUP.clone()),
            initial_double_click: None,
        },
    )
}

pub fn item_actor_for(
    payload_manager: &PayloadManager,
    asset_server: &AssetServer,
    scenes: &Assets<Scene>,
    item: Entity,
) -> impl Bundle {
    (
        item_actor(payload_manager, asset_server, scenes),
        ItemActor(item),
    )
}

pub fn item_actor_of(
    payload_manager: &PayloadManager,
    asset_server: &AssetServer,
    scenes: &Assets<Scene>,
    item: impl Bundle,
) -> impl Bundle {
    (
        item_actor(payload_manager, asset_server, scenes),
        Children::spawn(SpawnItem(item)),
    )
}
