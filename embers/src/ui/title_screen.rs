use super::loading_screen::{DimensionEntryContext, Load};
use super::{
    ActiveOverlay, NodeInteraction, RootNode, TextureAnimation, TextureScaling, text,
    text_button_node,
};
use crate::dim::embers;
use crate::pld::PayloadManager;
use bevy::color::palettes::basic::{WHITE, YELLOW};
use bevy::input_focus::tab_navigation::{TabGroup, TabIndex};
use bevy::prelude::*;
use bevy::sprite::Text2dShadow;
use bevy::ui::InteractionDisabled;

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
    commands.spawn((
        ChildOf(*root_node),
        DespawnOnExit(ActiveOverlay::TitleScreen),
        Node {
            width: percent(100),
            height: percent(100),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        TabGroup::new(0),
    )).with_children(|parent| {
        parent.spawn((
            Node {
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                ..default()
            },
        )).with_children(|parent| {
            parent.spawn((
                Text::new("Embers"),
                TextFont { ..default() },
                Text2dShadow::default(),
                TextColor(YELLOW.into())
            ));
            parent.spawn((
                text_button_node(&payload_manager, &asset_server, &images, &texture_atlas_layouts, &texture_animations, &texture_scalings, &fonts, "Play"),
                TabIndex(0),
            )).observe(|_interaction: On<NodeInteraction>, mut commands: Commands| {
                commands.trigger(Load::EnterDimension(DimensionEntryContext::EnterWorld, embers::LOBBY.clone()));
            });
            parent.spawn((
                text_button_node(&payload_manager, &asset_server, &images, &texture_atlas_layouts, &texture_animations, &texture_scalings, &fonts, "Options"),
                TabIndex(1),
            )).observe(|_interaction: On<NodeInteraction>, mut next_overlay: ResMut<NextState<ActiveOverlay>>| next_overlay.set(ActiveOverlay::OptionsMain));
            parent.spawn((
                text_button_node(&payload_manager, &asset_server, &images, &texture_atlas_layouts, &texture_animations, &texture_scalings, &fonts, "Quit"),
                InteractionDisabled,
                TabIndex(2),
            )).observe(|_interaction: On<NodeInteraction>, mut app_exit_writer: MessageWriter<AppExit>| {
                app_exit_writer.write(AppExit::Success);
            });
        });
        parent.spawn((
            Node {
                bottom: px(-3),
                left: px(2),
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Start,
                ..default()
            },
            text(
                &payload_manager,
                &asset_server,
                &fonts,
                format!("Embers {}", crate::VERSION),
                WHITE,
                14.
            ),
        ));
    });
}

fn fina() {}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(ActiveOverlay::TitleScreen), init)
        .add_systems(OnExit(ActiveOverlay::TitleScreen), fina);
}
