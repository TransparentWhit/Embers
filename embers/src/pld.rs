//! *Payload*(pld)*s* are resources that the game uses during execution, such as assets or game data.
//! *Shipment*(shp)*s* are their processed counterpart.

pub mod atlas;
pub mod meta;

use crate::pld::meta::ReloadMetadata;
use crate::utils::{ConstHashSet, Namespaced, NamespacedKey, const_hash_set};
use crate::{ASSETS_ROOT, GameState};
use bevy::app::App;
use bevy::asset::{AssetPath, LoadedFolder};
use bevy::prelude::*;
use std::sync::{LazyLock, Mutex};

#[derive(Eq, PartialEq, Hash, Clone)]
pub struct PayloadScope<'scope> {
    root: AssetPath<'scope>,
    textures_root: AssetPath<'scope>,
    models_root: AssetPath<'scope>,
}

impl<'scope> PayloadScope<'scope> {
    pub fn new(root: impl Into<AssetPath<'scope>>) -> Self {
        let root = root.into();
        Self {
            textures_root: root.resolve("textures").unwrap(),
            models_root: root.resolve("models").unwrap(),
            root,
        }
    }
    #[inline]
    pub fn actor_scene(
        &self,
        asset_server: &AssetServer,
        key: &NamespacedKey,
        label: usize,
    ) -> Handle<Scene> {
        self.scene(
            asset_server,
            &*format!("actors/{}/{}", key.namespace(), key.key()),
            label,
        )
    }
    #[inline]
    pub fn animate_actor(
        &self,
        animation_player: &mut AnimationPlayer,
        asset_server: &AssetServer,
        key: &NamespacedKey,
        label: usize,
    ) -> AnimationGraphHandle {
        self.animate(
            animation_player,
            asset_server,
            &*format!("actors/{}/{}", key.namespace(), key.key()),
            label,
        )
    }
    #[inline]
    pub fn ui_image<'path>(
        &self,
        asset_server: &AssetServer,
        path: impl Into<&'path str>,
    ) -> ImageNode {
        self.image_node(asset_server, &*format!("ui/{}", path.into()))
            .with_mode(NodeImageMode::Stretch)
    }
    #[inline]
    pub fn item_image(&self, asset_server: &AssetServer, key: &NamespacedKey) -> ImageNode {
        self.image_node(
            asset_server,
            &*format!("items/{}/{}", key.namespace(), key.key()),
        )
        .with_mode(NodeImageMode::Stretch)
    }
    #[inline]
    pub fn default_model(&self, asset_server: &AssetServer) -> Handle<Scene> {
        self.model(asset_server, "missingno", GltfAssetLabel::Scene(0))
    }
    #[inline]
    fn image_node<'path>(
        &self,
        asset_server: &AssetServer,
        path: impl Into<&'path str>,
    ) -> ImageNode {
        let (image, atlas) = self.texture(asset_server, path);
        if let Some(atlas) = atlas {
            ImageNode::from_atlas_image(image, atlas)
        } else {
            ImageNode::new(image)
        }
    }
    #[inline]
    fn scene<'path>(
        &self,
        asset_server: &AssetServer,
        path: impl Into<&'path str>,
        label: usize,
    ) -> Handle<Scene> {
        self.model(asset_server, path, GltfAssetLabel::Scene(label))
    }
    #[inline]
    fn animate<'path>(
        &self,
        animation_player: &mut AnimationPlayer,
        asset_server: &AssetServer,
        path: impl Into<&'path str>,
        label: usize,
    ) -> AnimationGraphHandle {
        let (graph, index) = AnimationGraph::from_clip(self.animation(asset_server, path, label));
        animation_player.play(index).repeat();
        AnimationGraphHandle(asset_server.add(graph))
    }
    #[inline]
    fn animation<'path>(
        &self,
        asset_server: &AssetServer,
        path: impl Into<&'path str>,
        label: usize,
    ) -> Handle<AnimationClip> {
        self.model(asset_server, path, GltfAssetLabel::Animation(label))
    }
    #[inline]
    fn image<'path>(
        &self,
        asset_server: &AssetServer,
        path: impl Into<&'path str>,
    ) -> Handle<Image> {
        asset_server.load(
            self.textures_root
                .resolve(&format!("{}.png", path.into()))
                .unwrap(),
        )
    }
    #[inline]
    fn texture<'path>(
        &self,
        asset_server: &AssetServer,
        path: impl Into<&'path str>,
    ) -> (Handle<Image>, Option<TextureAtlas>) {
        let path = path.into();
        let atlas_path = self
            .textures_root
            .resolve(&format!("{}.atlas.toml", path))
            .unwrap();
        (
            asset_server.load(
                self.textures_root
                    .resolve(&format!("{}.png", path))
                    .unwrap(),
            ),
            if ASSETS_ROOT.get().unwrap().join(atlas_path.path()).exists() {
                Some(TextureAtlas::from(asset_server.load(atlas_path)))
            } else {
                None
            },
        )
    }
    #[inline]
    fn model<'path, M: Asset>(
        &self,
        asset_server: &AssetServer,
        path: impl Into<&'path str>,
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

pub static GLOBAL_PAYLOADS: LazyLock<PayloadScope> = LazyLock::new(|| PayloadScope::new("global"));

fn load_global_payloads(mut asset_load_requests: MessageWriter<PayloadLoadRequest>) {
    asset_load_requests.write(PayloadLoadRequest::Scope(&GLOBAL_PAYLOADS));
}

#[derive(Message)]
pub enum PayloadLoadRequest {
    Scope(&'static PayloadScope<'static>),
}

#[derive(Message)]
pub struct PayloadLoadedMessage {}

#[derive(Message)]
pub enum PayloadUnloadRequest {
    Scope(&'static PayloadScope<'static>),
}

static LOADED_PAYLOADS: Mutex<ConstHashSet<UntypedHandle>> = Mutex::new(const_hash_set());

fn payload_load_request_listener(
    mut requests: MessageReader<PayloadLoadRequest>,
    asset_server: Res<AssetServer>,
    mut game_state: ResMut<NextState<GameState>>,
) {
    for request in requests.read() {
        LOADED_PAYLOADS.lock().unwrap().insert(match request {
            PayloadLoadRequest::Scope(scope) => {
                asset_server.load_folder(scope.root.clone()).untyped()
            }
        });
        game_state.set(GameState::Loading);
    }
}

fn payload_unload_request_listener(
    mut requests: MessageReader<PayloadUnloadRequest>,
    asset_server: Res<AssetServer>,
) {
    for request in requests.read() {
        match request {
            PayloadUnloadRequest::Scope(scope) => asset_server
                .get_path_id(scope.root.clone())
                .and_then(|untyped_id| asset_server.get_id_handle_untyped(untyped_id)),
        }
        .map(|handle| LOADED_PAYLOADS.lock().unwrap().remove(&handle));
    }
}

fn folder_loaded_listener(
    mut commands: Commands,
    mut folder_events: MessageReader<AssetEvent<LoadedFolder>>,
    mut messages: MessageWriter<PayloadLoadedMessage>,
) {
    for event in folder_events.read() {
        if let AssetEvent::LoadedWithDependencies { .. } = event {
            messages.write(PayloadLoadedMessage {});
            commands.trigger(ReloadMetadata);
        }
    }
}

pub(super) fn plugin(app: &mut App) {
    app.add_message::<PayloadLoadRequest>()
        .add_message::<PayloadUnloadRequest>()
        .add_message::<PayloadLoadedMessage>()
        .add_systems(Startup, load_global_payloads)
        .add_systems(
            Update,
            (
                payload_load_request_listener,
                payload_unload_request_listener,
            ),
        )
        .add_systems(Update, (folder_loaded_listener,))
        .add_plugins(atlas::plugin)
        .add_plugins(meta::plugin);
}
