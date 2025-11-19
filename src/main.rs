mod ui;
pub mod utils;
pub mod world;

use avian3d::PhysicsPlugins;
use avian3d::prelude::PhysicsSchedule;
use bevy::DefaultPlugins;
use bevy::prelude::*;
use bevy::winit::WINIT_WINDOWS;
use bevy_tnua::prelude::TnuaControllerPlugin;
use bevy_tnua_avian3d::TnuaAvian3dPlugin;
use std::path::MAIN_SEPARATOR;
use winit::window::Icon;

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
enum GameState {
    #[default]
    Loading,
    MainMenu,
    World,
}

// TODO: The initial loading logic is a bit messy. Someone refactor it later
// asset.rs initiates global asset loading at StartUp,
// when it completes it sends a message which gets received by loading_screen.rs,
// then somehow correctly switches to main screen because GameState and Loading happen to be in the correct default states
fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            file_path: format!("res{}assets", MAIN_SEPARATOR),
            ..default()
        }))
        .add_plugins(PhysicsPlugins::default())
        .add_plugins(TnuaControllerPlugin::new(PhysicsSchedule))
        .add_plugins(TnuaAvian3dPlugin::new(PhysicsSchedule))
        .add_plugins(ui::plugin)
        .add_plugins(utils::assets::assets_plugin)
        .add_plugins(world::plugin)
        .add_systems(
            Startup,
            |asset_images: Res<Assets<Image>>,
             asset_server: Res<AssetServer>,
             mut windows: Query<(Entity, &mut Window)>| {
                WINIT_WINDOWS.with_borrow(|winit_windows| {
                    for (window_entity, mut window) in windows.iter_mut() {
                        window.visible = true;
                        if let Some(winit_window) = winit_windows.get_window(window_entity) {
                            if let Some(window_icon) =
                                asset_images.get(&asset_server.load("global/icon.png"))
                            {
                                winit_window.set_window_icon(
                                    Icon::from_rgba(
                                        window_icon.data.clone().unwrap(),
                                        window_icon.width(),
                                        window_icon.height(),
                                    )
                                    .ok(),
                                );
                            }
                        }
                    }
                });
            },
        )
        .init_state::<GameState>()
        .run();
}
