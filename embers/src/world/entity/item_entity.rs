use crate::world::entity::entity;
use avian3d::prelude::*;
use bevy::prelude::*;

#[derive(Component, Debug)]
pub struct ItemEntity(pub Entity);

pub fn item_entity(item: Entity) -> impl Bundle {
    (
        entity(),
        ItemEntity(item),
        Collider::cuboid(0.25, 0.25, 0.25),
        RigidBody::Dynamic,
    )
}
