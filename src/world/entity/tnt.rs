use std::sync::{LazyLock, OnceLock};
use avian3d::prelude::*;
use bevy::prelude::*;
use super::entity;

//static MODEL: OnceLock<SceneRoot> = OnceLock::new(|| SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/entities/tnt/tnt.glb"), )));
static HITBOX: LazyLock<Collider> = LazyLock::new(|| Collider::cuboid(1.0, 1.0, 1.0));

#[derive(Component)]
pub struct TNT {}

pub fn tnt() -> impl Bundle {
    (
        entity(),
        TNT {},
        //MODEL.get().unwrap().clone(),
        HITBOX.clone(),
        RigidBody::Dynamic,
    )
}
