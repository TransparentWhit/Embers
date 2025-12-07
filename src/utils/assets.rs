use crate::GameState;
use crate::utils::{ConstHashSet, Namespaced, NamespacedKey, const_hash_set};
use bevy::app::App;
use bevy::asset::{AssetPath, LoadedFolder};
use bevy::prelude::*;
use std::sync::{LazyLock, Mutex};

#[derive(Eq, PartialEq, Hash, Clone)]
pub struct AssetScope {
    root: AssetPath<'static>,
    images_root: AssetPath<'static>,
    models_root: AssetPath<'static>,
}
impl AssetScope {
    pub fn new(root: impl Into<AssetPath<'static>>) -> Self {
        let root = root.into();
        Self {
            images_root: root.resolve("images").unwrap(),
            models_root: root.resolve("models").unwrap(),
            root,
        }
    }
    #[inline]
    pub fn entity_scene(
        &self,
        asset_server: &AssetServer,
        key: &NamespacedKey,
        label: usize,
    ) -> Handle<Scene> {
        self.scene(
            asset_server,
            &*format!("entities/{}/{}", key.namespace(), key.key()),
            label,
        )
    }
    #[inline]
    pub fn animate_entity(
        &self,
        animation_player: &mut AnimationPlayer,
        asset_server: &AssetServer,
        key: &NamespacedKey,
        label: usize,
    ) -> AnimationGraphHandle {
        self.animate(
            animation_player,
            asset_server,
            &*format!("entities/{}/{}", key.namespace(), key.key()),
            label,
        )
    }
    #[inline]
    pub fn image_node<'a>(
        &self,
        asset_server: &AssetServer,
        path: impl Into<&'a str>,
    ) -> ImageNode {
        ImageNode::new(self.image(asset_server, &*format!("ui/{}.png", path.into())))
            .with_mode(NodeImageMode::Stretch)
    }
    #[inline]
    fn image<'a>(&self, asset_server: &AssetServer, path: impl Into<&'a str>) -> Handle<Image> {
        asset_server.load(self.images_root.resolve(path.into()).unwrap())
    }
    #[inline]
    fn scene<'a>(
        &self,
        asset_server: &AssetServer,
        path: impl Into<&'a str>,
        label: usize,
    ) -> Handle<Scene> {
        self.model(asset_server, path, GltfAssetLabel::Scene(label))
    }
    #[inline]
    fn animate<'a>(
        &self,
        animation_player: &mut AnimationPlayer,
        asset_server: &AssetServer,
        path: impl Into<&'a str>,
        label: usize,
    ) -> AnimationGraphHandle {
        let (graph, index) = AnimationGraph::from_clip(self.animation(asset_server, path, label));
        animation_player.play(index).repeat();
        AnimationGraphHandle(asset_server.add(graph))
    }
    #[inline]
    fn animation<'a>(
        &self,
        asset_server: &AssetServer,
        path: impl Into<&'a str>,
        label: usize,
    ) -> Handle<AnimationClip> {
        self.model(asset_server, path, GltfAssetLabel::Animation(label))
    }
    #[inline]
    fn model<'a, M: Asset>(
        &self,
        asset_server: &AssetServer,
        path: impl Into<&'a str>,
        label: GltfAssetLabel,
    ) -> Handle<M> {
        asset_server.load(
            label.from_asset(
                self.models_root
                    .resolve(&format!("{}.glb", path.into()))
                    .unwrap(),
            ),
        )
    }
}

pub static GLOBAL_ASSETS: LazyLock<AssetScope> = LazyLock::new(|| AssetScope::new("global"));

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

static LOADED_ASSETS: Mutex<ConstHashSet<UntypedHandle>> = Mutex::new(const_hash_set());

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

pub(crate) fn assets_plugin(app: &mut App) {
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
