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
        DespawnOnExit(ActiveOverlay::PauseScreen),
        text(
            &payload_manager,
            &asset_server,
            &fonts,
            "Paused",
            WHITE,
            14.,
        ),
    ));
}

fn fina() {}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(ActiveOverlay::PauseScreen), init)
        .add_systems(OnExit(ActiveOverlay::PauseScreen), fina);
}
