use super::dim::DimensionViewNode;
use super::{ActiveOverlay, text};
use bevy::color::palettes::basic::WHITE;
use bevy::prelude::*;

fn init(mut commands: Commands, dimension_view_node: Single<Entity, With<DimensionViewNode>>) {
    commands.spawn_scene(bsn! {
        #PauseScreen
        ChildOf({*dimension_view_node})
        DespawnOnExit<ActiveOverlay>(ActiveOverlay::PauseScreen)
        text("Paused", WHITE, 20.)
    });
}

fn fina() {}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(ActiveOverlay::PauseScreen), init)
        .add_systems(OnExit(ActiveOverlay::PauseScreen), fina);
}
