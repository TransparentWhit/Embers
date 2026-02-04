pub mod dim;
pub mod loading_screen;
pub mod main_menu;

use bevy::ecs::system::NonSendMarker;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::winit::WINIT_WINDOWS;
use serde::Deserialize;
use winit::window::Icon;

fn ui_button(label: impl Into<String>) -> impl Bundle {
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
        children![(Text::new(label))],
    )
}

const BUTTON_BACKGROUND_DEFAULT: BackgroundColor = BackgroundColor(Color::srgb(0.1, 0.1, 0.1));

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
