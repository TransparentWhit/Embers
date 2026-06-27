pub mod dim;
pub mod gateway_menu;
pub mod heads_up_display;
pub mod inventory;
pub mod loading_screen;
pub mod main_menu;
pub mod options_main;
pub mod options_video;
pub mod pause_screen;
pub mod title_screen;

use crate::pld::{PayloadManager, font, resolve_optional_payload, ui_image_node};
use crate::utils::NamespacedKey;
use bevy::color::palettes::basic::WHITE;
use bevy::ecs::relationship::{RelatedSpawnerCommands, Relationship};
use bevy::ecs::system::NonSendMarker;
use bevy::input_focus::InputFocus;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::text::FontSmoothing;
use bevy::ui::InteractionDisabled;
use bevy::window::PrimaryWindow;
use bevy::winit::WINIT_WINDOWS;
use serde::Deserialize;
use std::sync::LazyLock;
use winit::window::Icon;

#[derive(Component)]
pub struct RootNode;

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum GameState {
    Dimension,
    #[default]
    MainMenu,
}

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum ActiveOverlay {
    GatewayMenu,
    HeadsUpDisplay,
    Inventory,
    #[default]
    LoadingScreen,
    OptionsAudio,
    OptionsControls,
    OptionsLanguage,
    OptionsMain,
    OptionsVideo,
    PauseScreen,
    TitleScreen,
}

fn process_escaping(
    keys: Res<ButtonInput<KeyCode>>,
    game_state: Res<State<GameState>>,
    active_overlay: Res<State<ActiveOverlay>>,
    mut next_overlay: ResMut<NextState<ActiveOverlay>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        match **active_overlay {
            ActiveOverlay::HeadsUpDisplay => next_overlay.set(ActiveOverlay::PauseScreen),
            ActiveOverlay::GatewayMenu | ActiveOverlay::Inventory | ActiveOverlay::PauseScreen => {
                next_overlay.set(ActiveOverlay::HeadsUpDisplay)
            }
            ActiveOverlay::OptionsAudio
            | ActiveOverlay::OptionsControls
            | ActiveOverlay::OptionsLanguage
            | ActiveOverlay::OptionsVideo => next_overlay.set(ActiveOverlay::OptionsMain),
            ActiveOverlay::OptionsMain => match **game_state {
                GameState::Dimension => next_overlay.set(ActiveOverlay::HeadsUpDisplay),
                GameState::MainMenu => next_overlay.set(ActiveOverlay::TitleScreen),
            },
            ActiveOverlay::LoadingScreen | ActiveOverlay::TitleScreen => {}
        }
    }
}

#[derive(Clone, Debug, EntityEvent, PartialEq)]
pub struct NodeInteraction<Ext: Send + Sync + 'static = ()> {
    pub entity: Entity,
    pub extra: Ext,
}

impl<Ext: Send + Sync + 'static> NodeInteraction<Ext> {
    pub fn new(entity: Entity, extra: Ext) -> Self {
        Self { entity, extra }
    }
}

#[derive(Clone, Debug, EntityEvent, PartialEq)]
pub struct WidgetStateUpdated(Entity);

fn trigger_default_node_interaction(
    mut commands: Commands,
    focus: Res<InputFocus>,
    keys: Res<ButtonInput<KeyCode>>,
    interactions: Query<
        (Entity, &Interaction),
        (
            Changed<Interaction>,
            Without<InteractionDisabled>,
            With<Button>,
        ),
    >,
) {
    if keys.just_pressed(KeyCode::Enter)
        && let Some(node) = focus.0
    {
        commands.trigger(NodeInteraction::new(node, ()));
        //commands.spawn((AudioPlayer::new(), PlaybackSettings::DESPAWN));
    }
    for (node, interaction) in interactions.iter() {
        match interaction {
            Interaction::Pressed => commands.trigger(NodeInteraction::new(node, ())),
            _ => commands.trigger(WidgetStateUpdated(node)),
        }
    }
}

static UI_FONT: LazyLock<NamespacedKey> = LazyLock::new(|| NamespacedKey::new_embers("polygon"));

fn text(
    payload_manager: &PayloadManager,
    asset_server: &AssetServer,
    fonts: &Assets<Font>,
    text: impl Into<String>,
    color: impl Into<Color>,
    size: f32,
) -> impl Bundle {
    (
        Text::new(text),
        TextColor(color.into()),
        TextFont::from_font_size(size)
            .with_font_smoothing(FontSmoothing::None)
            .with_font(font(payload_manager, asset_server, fonts, &UI_FONT).unwrap()),
    )
}

#[derive(Clone, Component, Copy, Debug, Default, Eq, PartialEq)]
#[require(Hovered)]
struct ButtonWidget;

//fn button<E: EntityEvent, C: Component>(_event: On<E, C>, button: Query<, (With<ButtonWidget>)>) {}

pub trait RelatedSpawnerCommandsExt<R: Relationship> {
    fn spawn_hypertext(
        &mut self,
        payload_manager: &PayloadManager,
        asset_server: &AssetServer,
        fonts: &Assets<Font>,
        label: impl Into<String>,
        extra: impl Bundle,
    ) -> EntityCommands<'_>;
    fn spawn_text_button(
        &mut self,
        payload_manager: &PayloadManager,
        asset_server: &AssetServer,
        images: &Assets<Image>,
        texture_atlas_layouts: &Assets<TextureAtlasLayout>,
        texture_animations: &Assets<TextureAnimation>,
        texture_scalings: &Assets<TextureScaling>,
        fonts: &Assets<Font>,
        label: impl Into<String>,
        extra: impl Bundle,
    ) -> EntityCommands<'_>;
}

impl<R: Relationship> RelatedSpawnerCommandsExt<R> for RelatedSpawnerCommands<'_, R> {
    fn spawn_hypertext(
        &mut self,
        payload_manager: &PayloadManager,
        asset_server: &AssetServer,
        fonts: &Assets<Font>,
        label: impl Into<String>,
        extra: impl Bundle,
    ) -> EntityCommands<'_> {
        self.spawn((
            Button,
            Node {
                width: px(200),
                height: px(20),
                margin: UiRect::all(px(2)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            text(payload_manager, asset_server, fonts, label, WHITE, 14.),
            Hovered::default(),
            extra,
        ))
    }
    fn spawn_text_button(
        &mut self,
        payload_manager: &PayloadManager,
        asset_server: &AssetServer,
        images: &Assets<Image>,
        texture_atlas_layouts: &Assets<TextureAtlasLayout>,
        texture_animations: &Assets<TextureAnimation>,
        texture_scalings: &Assets<TextureScaling>,
        fonts: &Assets<Font>,
        label: impl Into<String>,
        extra: impl Bundle,
    ) -> EntityCommands<'_> {
        let mut commands = self.spawn((
            Button,
            Node {
                width: px(200),
                height: px(20),
                margin: UiRect::all(px(2)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ui_image_node(
                payload_manager,
                asset_server,
                images,
                texture_atlas_layouts,
                texture_animations,
                texture_scalings,
                "widgets/button",
            ),
            Hovered::default(),
            children![text(
                payload_manager,
                asset_server,
                fonts,
                label,
                WHITE,
                14.
            )],
            extra,
        ));
        commands.observe(|on_: On<Insert, Hovered>| {});
        commands
    }
}

fn text_button_node(
    payload_manager: &PayloadManager,
    asset_server: &AssetServer,
    images: &Assets<Image>,
    texture_atlas_layouts: &Assets<TextureAtlasLayout>,
    texture_animations: &Assets<TextureAnimation>,
    texture_scalings: &Assets<TextureScaling>,
    fonts: &Assets<Font>,
    label: impl Into<String>,
) -> impl Bundle {
    (
        Button,
        Node {
            width: px(200),
            height: px(20),
            margin: UiRect::all(px(2)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        ui_image_node(
            payload_manager,
            asset_server,
            images,
            texture_atlas_layouts,
            texture_animations,
            texture_scalings,
            "widgets/button",
        ),
        children![text(
            payload_manager,
            asset_server,
            fonts,
            label,
            WHITE,
            14.
        )],
    )
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case", tag = "mode")]
enum SliceScaling {
    Stretch,
    Tile { stretch_value: f32 },
}

impl From<SliceScaling> for SliceScaleMode {
    fn from(value: SliceScaling) -> Self {
        match value {
            SliceScaling::Stretch => Self::Stretch,
            SliceScaling::Tile { stretch_value } => Self::Tile { stretch_value },
        }
    }
}

#[derive(Asset, Clone, Debug, Default, Deserialize, TypePath)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum TextureScaling {
    Auto,
    #[default]
    Stretch,
    Sliced {
        border_width_min: f32,
        border_width_max: f32,
        border_height_min: f32,
        border_height_max: f32,
        center_scaling: SliceScaling,
        side_scaling: SliceScaling,
        max_corner_scale: f32,
    },
    Tiled {
        tile_x: bool,
        tile_y: bool,
        stretch_value: f32,
    },
}

impl From<&TextureScaling> for NodeImageMode {
    fn from(value: &TextureScaling) -> Self {
        match value {
            TextureScaling::Auto => Self::Auto,
            TextureScaling::Stretch => Self::Stretch,
            &TextureScaling::Sliced {
                border_width_min,
                border_width_max,
                border_height_min,
                border_height_max,
                center_scaling,
                side_scaling,
                max_corner_scale,
            } => Self::Sliced(TextureSlicer {
                border: BorderRect {
                    min_inset: Vec2::new(border_width_min, border_height_min),
                    max_inset: Vec2::new(border_width_max, border_height_max),
                },
                center_scale_mode: center_scaling.into(),
                sides_scale_mode: side_scaling.into(),
                max_corner_scale,
            }),
            &TextureScaling::Tiled {
                tile_x,
                tile_y,
                stretch_value,
            } => Self::Tiled {
                tile_x,
                tile_y,
                stretch_value,
            },
        }
    }
}

#[derive(Asset, Debug, Deserialize, TypePath)]
pub struct TextureAnimation {
    atlas_begin_index: usize,
    atlas_end_index: usize,
    frame_time_secs: f32,
}

#[derive(Clone, Component, Debug, Default, Eq, PartialEq)]
pub struct AnimatedTexture {
    animation: Handle<TextureAnimation>,
    timer: Option<Timer>,
}

impl AnimatedTexture {
    pub fn new(animation: Handle<TextureAnimation>) -> Self {
        Self {
            animation,
            timer: None,
        }
    }
}

fn run_animations(
    time: Res<Time>,
    atlas_animations: Res<Assets<TextureAnimation>>,
    mut animated: Query<(&mut ImageNode, &mut AnimatedTexture)>,
) {
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
                if atlas.index >= animation.atlas_end_index
                    || atlas.index < animation.atlas_begin_index
                {
                    atlas.index = animation.atlas_begin_index;
                }
            }
        }
    }
}

#[derive(Component, Debug)]
#[component(storage = "SparseSet")]
pub struct SetWindowIcon {
    pub image: Handle<Image>,
}

fn set_window_icons(
    mut commands: Commands,
    images: Res<Assets<Image>>,
    mut windows: Query<(Entity, &mut Window, &SetWindowIcon)>,
    _non_send_marker: NonSendMarker,
) {
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
}

pub(super) fn plugin(app: &mut App) {
    app.init_state::<GameState>()
        .init_state::<ActiveOverlay>()
        .init_asset::<TextureAnimation>()
        .init_asset::<TextureScaling>()
        .insert_resource(UiScale(3.))
        .add_systems(PreUpdate, process_escaping)
        .add_systems(Update, trigger_default_node_interaction)
        .add_systems(
            Update,
            (resolve_optional_payload::<TextureAnimation>(
                |commands, handle, _animation| {
                    commands.insert(AnimatedTexture::new(handle));
                },
                |commands| {
                    commands.remove::<AnimatedTexture>();
                },
            ),),
        ) // TODO use bsn after Bevy 0.19
        .add_systems(Update, run_animations)
        .add_systems(Update, set_window_icons)
        .add_systems(
            PreStartup,
            |mut commands: Commands,
             asset_server: Res<AssetServer>,
             primary_window: Single<Entity, With<PrimaryWindow>>| {
                commands.entity(*primary_window).insert(SetWindowIcon {
                    image: asset_server.load("embedded://embers/icon.png"),
                });
            },
        )
        .add_plugins((
            dim::plugin,
            heads_up_display::plugin,
            inventory::plugin,
            gateway_menu::plugin,
            loading_screen::plugin,
            main_menu::plugin,
            options_main::plugin,
            pause_screen::plugin,
            title_screen::plugin,
        ));
}
