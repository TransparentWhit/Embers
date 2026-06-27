use super::{GameState, RootNode};
use bevy::prelude::*;

fn init(mut commands: Commands) {
    commands.spawn((
        RootNode,
        DespawnOnExit(GameState::MainMenu),
        Camera2d,
        Node {
            width: percent(100),
            height: percent(100),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
    ));
}

fn fina() {}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::MainMenu), init)
        .add_systems(OnExit(GameState::MainMenu), fina);
}
