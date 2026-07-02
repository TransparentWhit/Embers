use super::dim::DimensionRootNode;
use super::{ActiveOverlay, text};
use bevy::color::palettes::basic::WHITE;
use bevy::prelude::*;

fn init(mut commands: Commands, dimension_root_node: Single<Entity, With<DimensionRootNode>>) {
    commands.spawn_scene(bsn! {
        ChildOf({*dimension_root_node})
        DespawnOnExit<ActiveOverlay>(ActiveOverlay::GatewayMenu)
        text("Gateway", WHITE, 20.)
    });
}

fn fina() {}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(ActiveOverlay::GatewayMenu), init)
        .add_systems(OnExit(ActiveOverlay::GatewayMenu), fina);
}
