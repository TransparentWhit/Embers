//! *Payload*(pld)*s* are resources that the game uses during execution, such as assets or game data.

pub mod meta;

use crate::GameState;
use crate::pld::meta::ReloadMetadata;
use crate::utils::{ConstHashSet, Namespaced, NamespacedKey, const_hash_set};
use anyhow::Error;
use bevy::app::App;
use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, AssetPath, LoadContext, LoadedFolder};
use bevy::prelude::*;
use serde::Deserialize;
use std::marker::PhantomData;
use std::sync::{LazyLock, Mutex};
use toml::from_slice;

pub struct MetadataLoader<T: Asset + for<'de> Deserialize<'de>>(
    &'static [&'static str],
    PhantomData<T>,
);

impl<T: Asset + for<'de> Deserialize<'de>> MetadataLoader<T> {
    pub fn new(extensions: &'static [&'static str]) -> Self {
        Self(extensions, PhantomData)
    }
}

impl<T: Asset + for<'de> Deserialize<'de>> AssetLoader for MetadataLoader<T> {
    type Asset = T;
    type Settings = ();
    type Error = Error;
    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        Ok(from_slice(&bytes)?)
    }
    fn extensions(&self) -> &[&str] {
        self.0
    }
}

#[derive(Eq, PartialEq, Hash, Clone)]
pub struct PayloadScope<'scope> {
    root: AssetPath<'scope>,
    images_root: AssetPath<'scope>,
    models_root: AssetPath<'scope>,
    items_root: AssetPath<'scope>,
}

impl<'scope> PayloadScope<'scope> {
    pub fn new(root: impl Into<AssetPath<'scope>>) -> Self {
        let root = root.into();
        Self {
            images_root: root.resolve("images").unwrap(),
            models_root: root.resolve("models").unwrap(),
            items_root: root.resolve("items").unwrap(),
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
    pub fn image_node<'path>(
        &self,
        asset_server: &AssetServer,
        path: impl Into<&'path str>,
    ) -> ImageNode {
        ImageNode::new(self.image(asset_server, &*format!("ui/{}.png", path.into())))
            .with_mode(NodeImageMode::Stretch)
    }
    #[inline]
    pub fn item_image(&self, asset_server: &AssetServer, key: &NamespacedKey) -> ImageNode {
        ImageNode::new(self.image(
            asset_server,
            &*format!("items/{}/{}.png", key.namespace(), key.key()),
        ))
        .with_mode(NodeImageMode::Stretch)
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
        asset_server.load(self.images_root.resolve(path.into()).unwrap())
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
    reload_metadata: Res<ReloadMetadata>,
    mut folder_events: MessageReader<AssetEvent<LoadedFolder>>,
    mut messages: MessageWriter<PayloadLoadedMessage>,
) {
    for event in folder_events.read() {
        if let AssetEvent::LoadedWithDependencies { .. } = event {
            messages.write(PayloadLoadedMessage {});
            commands.run_system(reload_metadata.system_id());
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
        .add_plugins(meta::plugin);
}
