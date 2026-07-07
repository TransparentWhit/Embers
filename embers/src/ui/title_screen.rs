use super::loading_screen::{DimensionEntryContext, Load};
use super::{ActiveOverlay, NodeInteraction, RootNode, text, text_button};
use crate::dim::embers;
use crate::pld::ui_image_node;
use bevy::color::palettes::css::WHITE;
use bevy::input_focus::tab_navigation::{TabGroup, TabIndex};
use bevy::prelude::*;
use bevy::ui::auto_directional_navigation::AutoDirectionalNavigation;

fn init(mut commands: Commands, root_node: Single<Entity, With<RootNode>>) {
    commands.spawn_scene(bsn! {
        ChildOf({*root_node})
        DespawnOnExit<ActiveOverlay>(ActiveOverlay::TitleScreen)
        Node {
            width: percent(100),
            height: percent(100),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
        }
        TabGroup::new(0)
        Children [
            (
                Node {
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                }
                Children [
                    (
                        Node {
                            width: px(256),
                            height: px(64),
                        }
                        ui_image_node(cfg_select! {
                            debug_assertions => "title_dev",
                            _ => "title",
                        })
                    ),
                    (
                        text_button("Play", |_interaction: On<NodeInteraction>, mut commands: Commands| {
                            commands.trigger(Load::EnterDimension(DimensionEntryContext::EnterWorld, embers::LOBBY.clone()));
                        })
                        AutoDirectionalNavigation
                        TabIndex(0)
                    ),
                    (
                        text_button("Options", |_interaction: On<NodeInteraction>, mut next_overlay: ResMut<NextState<ActiveOverlay>>| {
                            next_overlay.set(ActiveOverlay::OptionsMain);
                        })
                        AutoDirectionalNavigation
                        TabIndex(1)
                    ),
                    (
                        text_button("Quit", |_interaction: On<NodeInteraction>, mut app_exit_writer: MessageWriter<AppExit>| {
                            app_exit_writer.write(AppExit::Success);
                        })
                        AutoDirectionalNavigation
                        TabIndex(2)
                    ),
                ]
            ),
            (
                Node {
                    bottom: px(-3),
                    left: px(2),
                    position_type: PositionType::Absolute,
                    justify_content: JustifyContent::Start,
                }
                text(format!("Embers {}", crate::VERSION), WHITE, 14.)
            ),
        ]
    });
}

fn fina() {}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(ActiveOverlay::TitleScreen), init)
        .add_systems(OnExit(ActiveOverlay::TitleScreen), fina);
}
