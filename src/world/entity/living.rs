
use bevy::prelude::*;

#[derive(Component)]
pub struct LivingEntity {
    pub health: f32,
}

#[macro_export]
macro_rules! living_entity {
    [$($extra: expr),* $(,)?] => {$crate::entity![
        avian3d::dynamics::rigid_body::RigidBody::Kinematic
        $(, $extra)*
    ]};
}
