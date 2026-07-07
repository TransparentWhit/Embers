use super::{ActiveOverlay, NodeInteraction, RootNode, text, text_button};
use bevy::color::palettes::basic::WHITE;
use bevy::prelude::*;

fn init(mut commands: Commands, root_node: Single<Entity, With<RootNode>>) {
    commands.spawn_scene(bsn! {
        #OptionsMain
        ChildOf({*root_node})
        DespawnOnExit<ActiveOverlay>(ActiveOverlay::OptionsMain)
        Node {
            display: Display::Grid,
            grid_auto_flow: GridAutoFlow::Row,
            grid_template_columns: vec![GridTrack::auto(), GridTrack::auto()],
            grid_row: GridPlacement::auto(),
        }
        Children [
            (
                #OptionsLabel
                Node {
                    grid_column: GridPlacement::span(2),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                }
                text("Options", WHITE, 14.)
            ),
            (
                #AudioButton
                text_button("Audio", |_interaction: On<NodeInteraction>, mut next_overlay: ResMut<NextState<ActiveOverlay>>| {
                    next_overlay.set(ActiveOverlay::OptionsAudio)
                })
            ),
            (
                #ControlsButton
                text_button("Controls", |_interaction: On<NodeInteraction>, mut next_overlay: ResMut<NextState<ActiveOverlay>>| {
                    next_overlay.set(ActiveOverlay::OptionsControls)
                })
            ),
            (
                #LanguageButton
                text_button("Language", |_interaction: On<NodeInteraction>, mut next_overlay: ResMut<NextState<ActiveOverlay>>| {
                    next_overlay.set(ActiveOverlay::OptionsLanguage)
                })
            ),
            (
                #VideoButton
                text_button("Video", |_interaction: On<NodeInteraction>, mut next_overlay: ResMut<NextState<ActiveOverlay>>| {
                    next_overlay.set(ActiveOverlay::OptionsVideo)
                })
            ),
            (
                #OptionsDoneButton
                Node {
                    grid_column: GridPlacement::span(2),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                }
                text_button("Done", |_interaction: On<NodeInteraction>, mut keyboard_input: ResMut<ButtonInput<KeyCode>>| {
                    keyboard_input.press(KeyCode::Escape)
                })
            ),
        ]
    });
}

fn fina() {}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(ActiveOverlay::OptionsMain), init)
        .add_systems(OnExit(ActiveOverlay::OptionsMain), fina);
}
