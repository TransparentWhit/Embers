use crate::ui::*;
use crate::{GameState, ui, ui_button};
use bevy::color::palettes::basic::YELLOW;
use bevy::prelude::*;
use bevy::sprite::Text2dShadow;

#[derive(States, Clone, Copy, Default, Eq, PartialEq, Debug, Hash)]
enum MainMenuState {
    Main,
    Options,
    #[default]
    Disabled,
}

#[derive(Component)]
enum MainMenuButton {
    Play,
    Options,
    Quit,
}

pub fn init(mut commands: Commands) {
    commands.spawn((
        DespawnOnExit(GameState::MainMenu),
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
                    Text::new("Embers"),
                    TextFont { ..default() },
                    Text2dShadow::default(),
                    TextColor(YELLOW.into())
                ),
                ui_button!("Play", MainMenuButton::Play),
                ui_button!("Options", MainMenuButton::Options),
                ui_button!("Quit", MainMenuButton::Quit),
            ]
        )],
    ));
}
pub fn fina(mut commands: Commands) {}

fn menu_action(
    interaction_query: Query<(&Interaction, &MainMenuButton), (Changed<Interaction>, With<Button>)>,
    mut app_exit_writer: MessageWriter<AppExit>,
    mut menu_state: ResMut<NextState<MainMenuState>>,
    mut game_state: ResMut<NextState<GameState>>,
) {
    for (interaction, menu_button_action) in &interaction_query {
        if *interaction == Interaction::Pressed {
            match menu_button_action {
                MainMenuButton::Play => {
                    game_state.set(GameState::World);
                    menu_state.set(MainMenuState::Disabled);
                }
                MainMenuButton::Options => menu_state.set(MainMenuState::Options),
                MainMenuButton::Quit => {
                    app_exit_writer.write(AppExit::Success);
                }
            }
        }
    }
}

pub fn main_menu_plugin(app: &mut App) {
    app.init_state::<MainMenuState>()
        .add_systems(OnEnter(GameState::MainMenu), init)
        .add_systems(Update, menu_action.run_if(in_state(GameState::MainMenu)))
        .add_systems(OnExit(GameState::MainMenu), fina);
}
