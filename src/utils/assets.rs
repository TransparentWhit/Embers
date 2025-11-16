use crate::GameState;
use bevy::app::App;
use bevy::asset::LoadedFolder;
use bevy::prelude::*;
use std::collections::HashSet;
use std::hash::{BuildHasherDefault, DefaultHasher};
use std::sync::Mutex;

fn load_global_assets(
    mut asset_load_requests: MessageWriter<AssetLoadRequest>,
) {
    asset_load_requests.write(AssetLoadRequest::Folder {
        path: "global".to_string(),
    });
}

#[derive(Message)]
pub enum AssetLoadRequest {
    Folder {
        path: String,
    }
}

#[derive(Message)]
pub struct AssetLoadedMessage {}

#[derive(Message)]
pub enum AssetUnloadRequest {
    Folder {
        handle: Handle<LoadedFolder>,
    }
}

static LOADED_ASSETS: Mutex<HashSet<UntypedHandle, BuildHasherDefault<DefaultHasher>>> = Mutex::new(HashSet::with_hasher(BuildHasherDefault::new()));

fn asset_load_request_listener(
    mut requests: MessageReader<AssetLoadRequest>,
    asset_server: Res<AssetServer>,
    mut game_state: ResMut<NextState<GameState>>,
) {
    for request in requests.read() {
        LOADED_ASSETS.lock().unwrap().insert(
            match request {
                AssetLoadRequest::Folder { path } => asset_server.load_folder(path).untyped()
            }
        );
        game_state.set(GameState::Loading);
    }
}
fn asset_unload_request_listener(
    mut requests: MessageReader<AssetUnloadRequest>,
) {
    for request in requests.read() {
        LOADED_ASSETS.lock().unwrap().remove(
            &match request {
                AssetUnloadRequest::Folder { handle } => handle.clone().untyped()
            }
        );
    }
}

fn folder_loaded_listener(
    mut folder_events: MessageReader<AssetEvent<LoadedFolder>>,
    mut messages: MessageWriter<AssetLoadedMessage>,
) {
    for event in folder_events.read() {
        if let AssetEvent::LoadedWithDependencies { .. } = event {
            messages.write(AssetLoadedMessage {});
        }
    }
}

pub fn assets_plugin(app: &mut App) {
    app
        .add_message::<AssetLoadRequest>()
        .add_message::<AssetUnloadRequest>()
        .add_message::<AssetLoadedMessage>()
        .add_systems(Startup, load_global_assets)
        .add_systems(Update, (
            asset_load_request_listener,
            asset_unload_request_listener,
        ))
        .add_systems(Update, (
            folder_loaded_listener,
        ))
    ;
}
