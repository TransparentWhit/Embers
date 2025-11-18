use crate::GameState;
use crate::utils::{Namespaced, NamespacedKey};
use bevy::app::App;
use bevy::asset::{AssetPath, LoadedFolder};
use bevy::prelude::*;
use std::collections::HashSet;
use std::hash::{BuildHasherDefault, DefaultHasher};
use std::sync::{LazyLock, Mutex};

#[derive(Eq, PartialEq, Hash, Clone)]
pub struct AssetScope {
    root: AssetPath<'static>,
}
impl AssetScope {
    pub fn new(root: impl Into<AssetPath<'static>>) -> Self {
        Self { root: root.into() }
    }
    pub fn entity_model(&self, asset_server: &AssetServer, key: &NamespacedKey) -> Handle<Scene> {
        asset_server.load(
            self.root
                .resolve(&format!(
                    "models/entities/{}/{}.glb#Scene0",
                    key.namespace(),
                    key.key()
                ))
                .unwrap(),
        )
    }
}

pub static GLOBAL_ASSETS: LazyLock<AssetScope> = LazyLock::new(|| AssetScope {
    root: AssetPath::parse("global"),
});

fn load_global_assets(mut asset_load_requests: MessageWriter<AssetLoadRequest>) {
    asset_load_requests.write(AssetLoadRequest::Scope(&GLOBAL_ASSETS));
}

#[derive(Message)]
pub enum AssetLoadRequest {
    Scope(&'static AssetScope),
}

#[derive(Message)]
pub struct AssetLoadedMessage {}

#[derive(Message)]
pub enum AssetUnloadRequest {
    Scope(&'static AssetScope),
}

static LOADED_ASSETS: Mutex<HashSet<UntypedHandle, BuildHasherDefault<DefaultHasher>>> =
    Mutex::new(HashSet::with_hasher(BuildHasherDefault::new()));

fn asset_load_request_listener(
    mut requests: MessageReader<AssetLoadRequest>,
    asset_server: Res<AssetServer>,
    mut game_state: ResMut<NextState<GameState>>,
) {
    for request in requests.read() {
        LOADED_ASSETS.lock().unwrap().insert(match request {
            AssetLoadRequest::Scope(scope) => {
                asset_server.load_folder(scope.root.clone()).untyped()
            }
        });
        game_state.set(GameState::Loading);
    }
}
fn asset_unload_request_listener(
    mut requests: MessageReader<AssetUnloadRequest>,
    asset_server: Res<AssetServer>,
) {
    for request in requests.read() {
        match request {
            AssetUnloadRequest::Scope(scope) => asset_server
                .get_path_id(scope.root.clone())
                .and_then(|untyped_id| asset_server.get_id_handle_untyped(untyped_id)),
        }
        .map(|handle| LOADED_ASSETS.lock().unwrap().remove(&handle));
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
    app.add_message::<AssetLoadRequest>()
        .add_message::<AssetUnloadRequest>()
        .add_message::<AssetLoadedMessage>()
        .add_systems(Startup, load_global_assets)
        .add_systems(
            Update,
            (asset_load_request_listener, asset_unload_request_listener),
        )
        .add_systems(Update, (folder_loaded_listener,));
}
