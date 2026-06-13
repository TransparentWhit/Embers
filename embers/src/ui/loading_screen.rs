use crate::GameState;
use crate::pld::{GLOBAL_PAYLOADS, PayloadLoadedMessage};
use crate::ui::ui_text;
use bevy::app::App;
use bevy::color::palettes::css::YELLOW;
use bevy::prelude::*;

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
            Loading::World => GameState::Dimension,
        }
    }
}

pub fn set_next_state(next_state: GameState) {}

fn init(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        DespawnOnExit(GameState::Loading),
        Camera2d,
        Transform::default(),
        Node {
            width: percent(100),
            height: percent(100),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        children![
            (
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    ..default()
                },
                children![ui_text(&asset_server, "Loading...", YELLOW.into(), 20.)]
            ),
            (
                Node {
                    position_type: PositionType::Absolute,
                    height: px(32),
                    right: px(4),
                    bottom: px(4),
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::End,
                    align_items: AlignItems::Center,
                    ..default()
                },
                children![(
                    Node {
                        width: px(32),
                        height: px(32),
                        ..default()
                    },
                    GLOBAL_PAYLOADS.ui_image(&asset_server, "loading_indicator"),
                ),]
            )
        ],
    ));
}

fn payload_loaded_listener(
    mut messages: MessageReader<PayloadLoadedMessage>,
    loading: Res<State<Loading>>,
    mut game_state: ResMut<NextState<GameState>>,
) {
    for _message in messages.read() {
        game_state.set(loading.game_state());
    }
}

fn fina() {}

pub(super) fn plugin(app: &mut App) {
    app.init_state::<Loading>()
        .add_systems(OnEnter(GameState::Loading), init)
        .add_systems(Update, payload_loaded_listener)
        .add_systems(OnExit(GameState::Loading), fina);
}
