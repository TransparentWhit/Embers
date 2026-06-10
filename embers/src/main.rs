#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod dim;
pub mod input;
pub mod pld;
pub mod reg;
mod ui;
pub mod utils;

use crate::dim::Movements;
use crate::pld::{DelegatingAssetReader, PAYLOADS_SOURCE};
use avian3d::PhysicsPlugins;
use avian3d::prelude::PhysicsSchedule;
use bevy::DefaultPlugins;
use bevy::asset::UnapprovedPathMode;
use bevy::asset::io::file::FileAssetReader;
use bevy::asset::io::{AssetSourceBuilder, AssetSourceId};
use bevy::image::ImageSamplerDescriptor;
use bevy::log::{Level, LogPlugin};
use bevy::prelude::*;
use bevy::window::WindowTheme;
use bevy_hanabi::HanabiPlugin;
use bevy_tnua::prelude::TnuaControllerPlugin;
use bevy_tnua_avian3d::TnuaAvian3dPlugin;
use std::env::current_exe;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub static UNPROCESSED_ASSETS_ROOT: OnceLock<PathBuf> = OnceLock::new();

pub static ASSETS_ROOT: OnceLock<PathBuf> = OnceLock::new();

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
enum GameState {
    Dimension,
    #[default]
    Loading,
    MainMenu,
}

// TODO: The initial loading logic is a bit messy. Someone refactor it later
// asset.rs initiates global asset loading at StartUp,
// when it completes it sends a message which gets received by loading_screen.rs,
// then somehow correctly switches to main screen because GameState and Loading happen to be in the correct default states
fn main() {
    let mut app = App::new();
    app.add_plugins(LogPlugin {
        level: Level::INFO,
        ..default()
    });
    let mut current_path = current_exe().unwrap();
    while let Ok(destination) = current_path.read_link() {
        current_path = destination;
    }
    let find_resource_root = |folder, marker| {
        let mut path = current_path.clone();
        while path.pop() {
            let resources = path.join(folder);
            if resources.is_dir() && resources.join(marker).exists() {
                return Some(resources);
            }
        }
        None
    };
    #[cfg(debug_assertions)]
    UNPROCESSED_ASSETS_ROOT
        .set(
            find_resource_root("pld", ".embers_payload_root")
                .inspect(|path| info!("Found payload root: {}", path.display()))
                .unwrap_or_else(|| {
                    warn!("Could not find payload root!");
                    Path::new("pld").to_path_buf()
                }),
        )
        .unwrap();
    ASSETS_ROOT
        .set(
            find_resource_root("shp", ".embers_shipment_root")
                .inspect(|path| info!("Found shipment root: {}", path.display()))
                .unwrap_or_else(|| {
                    error!("Could not find shipment root!");
                    Path::new("shp").to_path_buf()
                }),
        )
        .unwrap();
    app.register_asset_source(
        AssetSourceId::Name(PAYLOADS_SOURCE.clone()),
        AssetSourceBuilder::new(|| {
            Box::new(DelegatingAssetReader::new(FileAssetReader::new(
                ASSETS_ROOT.get().unwrap().clone(),
            )))
        }),
    )
    .add_plugins(
        DefaultPlugins
            .build()
            .set(AssetPlugin {
                file_path: UNPROCESSED_ASSETS_ROOT
                    .get()
                    .unwrap()
                    .to_string_lossy()
                    .to_string(),
                processed_file_path: ASSETS_ROOT.get().unwrap().to_string_lossy().to_string(),
                mode: AssetMode::Processed,
                unapproved_path_mode: UnapprovedPathMode::Deny,
                ..default()
            })
            .set(ImagePlugin {
                default_sampler: ImageSamplerDescriptor::nearest(),
                ..default()
            })
            .disable::<LogPlugin>()
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Embers".to_string(),
                    window_theme: Some(WindowTheme::Dark),
                    visible: false,
                    ..default()
                }),
                ..default()
            }),
    )
    .add_plugins(HanabiPlugin)
    .add_plugins(
        PhysicsPlugins::default().with_collision_hooks::<dim::SourceExclusionCollisionHooks>(),
    )
    .add_plugins(TnuaControllerPlugin::<Movements>::new(PhysicsSchedule))
    .add_plugins(TnuaAvian3dPlugin::new(PhysicsSchedule))
    .add_plugins(dim::plugin)
    .add_plugins(input::plugin)
    .add_plugins(pld::plugin)
    .add_plugins(ui::plugin)
    .init_state::<GameState>()
    .run();
}
