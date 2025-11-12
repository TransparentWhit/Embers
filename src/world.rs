pub mod entity;

use bevy::prelude::Component;

/// Time of the day, within [0, 1).
#[derive(Component)]
pub struct Time(pub f32);
impl Default for Time {
    fn default() -> Self {
        Self(0.25)
    }
}
