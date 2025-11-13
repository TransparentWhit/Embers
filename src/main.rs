mod ui;
pub mod world;
mod utils;

use avian3d::PhysicsPlugins;
use bevy::DefaultPlugins;
use bevy::asset::uuid::Uuid;
use bevy::prelude::*;

#[derive(Component)]
struct UUID(Uuid);

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
enum GameState {
    #[default]
    MainMenu,
    World,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            file_path: "res/assets".to_string(),
            ..default()
        }))
        .add_plugins(PhysicsPlugins::default())
        .add_plugins(ui::main_menu::main_menu_plugin)
        .add_plugins(ui::world::world_plugin)
        /*.add_systems(Startup, |
            primary_window: Single<Entity, With<PrimaryWindow>>,
            asset_server: Res<AssetServer>,
            mut windows: NonSend<WinitWindows>,
        | {
            let icon_path = "icon.png";
            let icon_handle: Handle<Image> = asset_server.load(icon_path);
            let window_entity = primary_window.into_inner();
            if let Some(window) = windows.get_window(window_entity) {
                if let Ok(crate_root) = std::env::current_dir() {
                    let icon_path = crate_root.join("res/assets/icon.png");
                    if let Ok(icon_file) = std::fs::File::open(&icon_path) {
                        let reader = std::io::BufReader::new(icon_file);
                        if let Ok(image) = image::load(icon_handle., image::ImageFormat::Png) {
                            let image = image.into_rgba8();
                            let (width, height) = image.dimensions();
                            let rgba_data = image.into_raw();
                            if let Ok(icon) = Icon::from_rgba(rgba_data, width, height) {
                                window.set_window_icon(Some(icon));
                            }
                        }
                    }
                }
            }
        })*/
        .init_state::<GameState>()
        .run();
}
