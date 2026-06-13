pub mod dim;
pub mod loading_screen;
pub mod main_menu;

use crate::pld::{GLOBAL_PAYLOADS, resolve_optional_payload};
use crate::utils::NamespacedKey;
use bevy::color::palettes::basic::WHITE;
use bevy::ecs::system::NonSendMarker;
use bevy::prelude::*;
use bevy::text::FontSmoothing;
use bevy::window::PrimaryWindow;
use bevy::winit::WINIT_WINDOWS;
use serde::Deserialize;
use std::sync::LazyLock;
use winit::window::Icon;

static UI_FONT: LazyLock<NamespacedKey> = LazyLock::new(|| NamespacedKey::new_embers("polygon"));

const BUTTON_BACKGROUND_DEFAULT: BackgroundColor = BackgroundColor(Color::srgb(0.1, 0.1, 0.1));

fn ui_text(
    asset_server: &AssetServer,
    text: impl Into<String>,
    color: Color,
    size: f32,
) -> impl Bundle {
    (
        Text::new(text),
        TextColor(color),
        TextFont::from_font_size(size)
            .with_font_smoothing(FontSmoothing::None)
            .with_font(GLOBAL_PAYLOADS.font(asset_server, &UI_FONT)),
    )
}

fn ui_button(asset_server: &AssetServer, label: impl Into<String>) -> impl Bundle {
    (
        Button,
        Node {
            width: px(200),
            height: px(20),
            margin: UiRect::all(px(3)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BUTTON_BACKGROUND_DEFAULT,
        children![ui_text(asset_server, label, WHITE.into(), 20.)],
    )
}

#[derive(Asset, Deserialize, TypePath, Debug)]
pub struct TextureAtlasAnimation {
    begin_index: usize,
    end_index: usize,
    frame_time_secs: f32,
}

#[derive(Component, Clone, Debug, Default, Eq, PartialEq)]
pub struct AnimatedTextureAtlas {
    animation: Handle<TextureAtlasAnimation>,
    timer: Option<Timer>,
}

impl AnimatedTextureAtlas {
    pub fn new(animation: Handle<TextureAtlasAnimation>) -> Self {
        Self {
            animation,
            timer: None,
        }
    }
}

#[derive(Component, Debug)]
#[component(storage = "SparseSet")]
pub struct SetWindowIcon {
    pub image: Handle<Image>,
}

pub(super) fn plugin(app: &mut App) {
    app.init_asset::<TextureAtlasAnimation>()
        .insert_resource(UiScale(3.))
        .add_systems(
            Update,
            (
                resolve_optional_payload::<TextureAtlasLayout>(
                    |commands, handle| {
                        commands.queue(|mut entity: EntityWorldMut| {
                            entity
                                .get_mut::<ImageNode>()
                                .expect("Texture atlas layout is present without image node")
                                .texture_atlas = Some(TextureAtlas::from(handle));
                        });
                    },
                    |_commands| {},
                ),
                resolve_optional_payload::<TextureAtlasAnimation>(
                    |commands, handle| {
                        commands.insert(AnimatedTextureAtlas::new(handle));
                    },
                    |commands| {
                        commands.remove::<AnimatedTextureAtlas>();
                    },
                ),
            ),
        )
        .add_systems(
            Update,
            |time: Res<Time>,
             atlas_animations: Res<Assets<TextureAtlasAnimation>>,
             mut animated: Query<(&mut ImageNode, &mut AnimatedTextureAtlas)>| {
                for (mut image_node, mut animated_image_node) in animated.iter_mut() {
                    if let Some(animation) = atlas_animations.get(&animated_image_node.animation)
                        && let Some(atlas) = &mut image_node.texture_atlas
                    {
                        let timer = animated_image_node.timer.get_or_insert_with(|| {
                            Timer::from_seconds(animation.frame_time_secs, TimerMode::Repeating)
                        });
                        timer.tick(time.delta());
                        if timer.just_finished() {
                            atlas.index = atlas.index.wrapping_add(1);
                            if atlas.index >= animation.end_index
                                || atlas.index < animation.begin_index
                            {
                                atlas.index = animation.begin_index;
                            }
                        }
                    }
                }
            },
        )
        .add_systems(
            Update,
            |mut commands: Commands,
             images: Res<Assets<Image>>,
             mut windows: Query<(Entity, &mut Window, &SetWindowIcon)>,
             _non_send_marker: NonSendMarker| {
                WINIT_WINDOWS.with_borrow(|winit_windows| {
                    for (window_entity, mut window, set_window_icon) in windows.iter_mut() {
                        window.visible = true;
                        if let Some(winit_window) = winit_windows.get_window(window_entity) {
                            if let Some(window_icon) = images.get(&set_window_icon.image) {
                                winit_window.set_window_icon(
                                    Icon::from_rgba(
                                        window_icon.data.clone().unwrap(),
                                        window_icon.width(),
                                        window_icon.height(),
                                    )
                                    .ok(),
                                );
                                commands.entity(window_entity).remove::<SetWindowIcon>();
                            }
                        }
                    }
                })
            },
        )
        .add_systems(
            PreStartup,
            |mut commands: Commands,
             asset_server: Res<AssetServer>,
             primary_window: Single<Entity, With<PrimaryWindow>>| {
                commands.entity(*primary_window).insert(SetWindowIcon {
                    image: asset_server.load("global/icon.png"),
                });
            },
        )
        .add_plugins((loading_screen::plugin, main_menu::plugin, dim::plugin));
}
