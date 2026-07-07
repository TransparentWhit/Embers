use super::dim::DimensionViewNode;
use super::{ActiveOverlay, text};
use bevy::color::palettes::basic::WHITE;
use bevy::prelude::*;

fn init(mut commands: Commands, dimension_view_node: Single<Entity, With<DimensionViewNode>>) {
    commands.spawn_scene(bsn! {
        #InventoryNode
        ChildOf({*dimension_view_node})
        DespawnOnExit<ActiveOverlay>(ActiveOverlay::Inventory)
        text("Inventory", WHITE, 20.)
    });
}

fn fina() {}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(ActiveOverlay::Inventory), init)
        .add_systems(OnExit(ActiveOverlay::Inventory), fina);
}
