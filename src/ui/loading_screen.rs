use bevy::app::App;
use bevy::color::palettes::css::YELLOW;
use bevy::prelude::*;
use crate::GameState;
use crate::utils::assets::AssetLoadedMessage;

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum Loading {
    #[default]
    MainMenu,
    World,
}
impl Loading {
    fn game_state(&self) -> GameState {
        match self {
            Loading::MainMenu => GameState::MainMenu,
            Loading::World => GameState::World,
        }
    }
}

pub fn set_next_state(next_state: GameState) {}

fn init(mut commands: Commands) {
    commands.spawn((
        DespawnOnExit(GameState::Loading),
        Camera2d,
        Transform::default(),
        Node {
            width: Val::Percent(100f32),
            height: Val::Percent(100f32),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        children![(
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                ..default()
            },
            children![
                (
                    Text::new("Loading..."),
                    TextFont { ..default() },
                    TextColor(YELLOW.into())
                ),
            ]
        )],
    ));
}

fn asset_loaded_listener(
    mut messages: MessageReader<AssetLoadedMessage>,
    loading: Res<State<Loading>>,
    mut game_state: ResMut<NextState<GameState>>,
) {
    for _message in messages.read() {
        game_state.set(loading.game_state());
    }
}

fn fina() {}

pub fn loading_screen_plugin(app: &mut App) {
    app
        .init_state::<Loading>()
        .add_systems(OnEnter(GameState::Loading), init)
        .add_systems(Update, asset_loaded_listener)
        .add_systems(OnExit(GameState::Loading), fina)
    ;
}
