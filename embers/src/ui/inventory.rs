use super::dim::DimensionRootNode;
use super::{ActiveOverlay, text};
use crate::pld::PayloadManager;
use bevy::color::palettes::basic::WHITE;
use bevy::prelude::*;

fn init(
    mut commands: Commands,
    payload_manager: Res<PayloadManager>,
    asset_server: Res<AssetServer>,
    fonts: Res<Assets<Font>>,
    dimension_root_node: Single<Entity, With<DimensionRootNode>>,
) {
    commands.spawn((
        ChildOf(*dimension_root_node),
        DespawnOnExit(ActiveOverlay::Inventory),
        text(
            &payload_manager,
            &asset_server,
            &fonts,
            "Inventory",
            WHITE,
            20.,
        ),
    ));
}

fn fina() {}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(ActiveOverlay::Inventory), init)
        .add_systems(OnExit(ActiveOverlay::Inventory), fina);
}
