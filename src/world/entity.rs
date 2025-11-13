pub mod player;
pub mod projectile;
pub mod living;

use bevy::prelude::*;

#[derive(Component)]
pub struct Entity;

#[macro_export]
macro_rules! entity {
    [$($extra: expr),* $(,)?] => {(
        $crate::world::entity::Entity
        $(, $extra)*
    )};
}
