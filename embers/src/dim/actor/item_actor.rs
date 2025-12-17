use crate::dim::actor::actor;
use crate::dim::CollisionLayer;
use avian3d::prelude::*;
use bevy::prelude::*;

#[derive(Component, Debug)]
pub struct ItemActor(pub Entity);

pub fn item_actor(item: Entity) -> impl Bundle {
    (
        actor(),
        ItemActor(item),
        Collider::cuboid(0.25, 0.25, 0.25),
        RigidBody::Dynamic,
        CollisionLayers::new(CollisionLayer::PhantomActor, [CollisionLayer::Environment]),
    )
}
