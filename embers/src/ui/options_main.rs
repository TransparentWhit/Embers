use super::{
    ActiveOverlay, NodeInteraction, RootNode, TextureAnimation, TextureScaling, text,
    text_button_node,
};
use crate::pld::PayloadManager;
use bevy::color::palettes::basic::WHITE;
use bevy::prelude::*;

fn init(
    mut commands: Commands,
    payload_manager: Res<PayloadManager>,
    asset_server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    texture_atlas_layouts: Res<Assets<TextureAtlasLayout>>,
    texture_animations: Res<Assets<TextureAnimation>>,
    texture_scalings: Res<Assets<TextureScaling>>,
    fonts: Res<Assets<Font>>,
    root_node: Single<Entity, With<RootNode>>,
) {
    commands
        .spawn((
            ChildOf(*root_node),
            DespawnOnExit(ActiveOverlay::OptionsMain),
            Node {
                display: Display::Grid,
                grid_auto_flow: GridAutoFlow::Row,
                grid_template_columns: vec![GridTrack::auto(), GridTrack::auto()],
                grid_row: GridPlacement::auto(),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                Node {
                    grid_column: GridPlacement::span(2),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                text(
                    &payload_manager,
                    &asset_server,
                    &fonts,
                    "Options",
                    WHITE,
                    14.,
                ),
            ));
            parent
                .spawn((text_button_node(
                    &payload_manager,
                    &asset_server,
                    &images,
                    &texture_atlas_layouts,
                    &texture_animations,
                    &texture_scalings,
                    &fonts,
                    "Audio",
                ),))
                .observe(
                    |_interaction: On<NodeInteraction>,
                     mut next_overlay: ResMut<NextState<ActiveOverlay>>| {
                        next_overlay.set(ActiveOverlay::OptionsAudio)
                    },
                );
            parent
                .spawn((text_button_node(
                    &payload_manager,
                    &asset_server,
                    &images,
                    &texture_atlas_layouts,
                    &texture_animations,
                    &texture_scalings,
                    &fonts,
                    "Controls",
                ),))
                .observe(
                    |_interaction: On<NodeInteraction>,
                     mut next_overlay: ResMut<NextState<ActiveOverlay>>| {
                        next_overlay.set(ActiveOverlay::OptionsControls)
                    },
                );
            parent
                .spawn((text_button_node(
                    &payload_manager,
                    &asset_server,
                    &images,
                    &texture_atlas_layouts,
                    &texture_animations,
                    &texture_scalings,
                    &fonts,
                    "Language",
                ),))
                .observe(
                    |_interaction: On<NodeInteraction>,
                     mut next_overlay: ResMut<NextState<ActiveOverlay>>| {
                        next_overlay.set(ActiveOverlay::OptionsLanguage)
                    },
                );
            parent
                .spawn((text_button_node(
                    &payload_manager,
                    &asset_server,
                    &images,
                    &texture_atlas_layouts,
                    &texture_animations,
                    &texture_scalings,
                    &fonts,
                    "Video",
                ),))
                .observe(
                    |_interaction: On<NodeInteraction>,
                     mut next_overlay: ResMut<NextState<ActiveOverlay>>| {
                        next_overlay.set(ActiveOverlay::OptionsVideo)
                    },
                );
            parent.spawn((
                Node {
                    grid_column: GridPlacement::span(2),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                children![
                    (text_button_node(
                        &payload_manager,
                        &asset_server,
                        &images,
                        &texture_atlas_layouts,
                        &texture_animations,
                        &texture_scalings,
                        &fonts,
                        "Done"
                    ))
                ],
            ));
        });
}

fn fina() {}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(ActiveOverlay::OptionsMain), init)
        .add_systems(OnExit(ActiveOverlay::OptionsMain), fina);
}
