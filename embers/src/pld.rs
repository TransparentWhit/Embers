//! *Payload*(pld)*s* are resources that the game uses during execution, such as assets or game data.
//! *Shipment*(shp)*s* are their processed counterpart.

pub mod meta;

use crate::GameState;
use crate::pld::meta::ReloadMetadata;
use crate::ui::TextureAtlasAnimation;
use crate::utils::{Namespaced, NamespacedKey};
use atomicow::CowArc;
use bevy::app::App;
use bevy::asset::io::{AssetReader, AssetReaderError, ErasedAssetReader, PathStream, Reader};
use bevy::asset::{AssetPath, LoadState, LoadedFolder};
use bevy::prelude::*;
use derive_where::derive_where;
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, LazyLock};

pub static PAYLOADS_SOURCE: CowArc<str> = CowArc::Static("payloads");

pub struct DelegatingAssetReader {
    inner: Box<dyn ErasedAssetReader>,
    fallback: Option<Arc<DelegatingAssetReader>>,
}

impl DelegatingAssetReader {
    pub fn new(inner: impl ErasedAssetReader) -> Self {
        Self {
            inner: Box::new(inner),
            fallback: None,
        }
    }
    pub fn new_delegating(
        inner: impl ErasedAssetReader,
        fallback: Arc<DelegatingAssetReader>,
    ) -> Self {
        Self {
            inner: Box::new(inner),
            fallback: Some(fallback),
        }
    }
}

impl AssetReader for DelegatingAssetReader {
    #[inline]
    async fn read<'reader>(
        &'reader self,
        path: &'reader Path,
    ) -> Result<impl Reader + 'reader, AssetReaderError> {
        match self.inner.read(path).await {
            Ok(reader) => Ok(reader),
            Err(err) => {
                if matches!(err, AssetReaderError::NotFound(ref _path))
                    && let Some(ref fallback) = self.fallback
                {
                    ErasedAssetReader::read(fallback.as_ref(), path).await
                } else {
                    Err(err)
                }
            }
        }
    }
    #[inline]
    async fn read_meta<'reader>(
        &'reader self,
        path: &'reader Path,
    ) -> Result<impl Reader + 'reader, AssetReaderError> {
        match self.inner.read_meta(path).await {
            Ok(meta_reader) => Ok(meta_reader),
            Err(err) => {
                if matches!(err, AssetReaderError::NotFound(ref _path))
                    && let Some(ref fallback) = self.fallback
                {
                    ErasedAssetReader::read_meta(fallback.as_ref(), path).await
                } else {
                    Err(err)
                }
            }
        }
    }
    #[inline]
    async fn read_directory<'reader>(
        &'reader self,
        path: &'reader Path,
    ) -> Result<Box<PathStream>, AssetReaderError> {
        match self.inner.read_directory(path).await {
            Ok(directory) => Ok(directory),
            Err(err) => {
                if matches!(err, AssetReaderError::NotFound(ref _path))
                    && let Some(ref fallback) = self.fallback
                {
                    ErasedAssetReader::read_directory(fallback.as_ref(), path).await
                } else {
                    Err(err)
                }
            }
        }
    }
    #[inline]
    async fn is_directory<'reader>(
        &'reader self,
        path: &'reader Path,
    ) -> Result<bool, AssetReaderError> {
        match self.inner.is_directory(path).await {
            Ok(is_dir) => Ok(is_dir),
            Err(err) => {
                if matches!(err, AssetReaderError::NotFound(ref _path))
                    && let Some(ref fallback) = self.fallback
                {
                    ErasedAssetReader::is_directory(fallback.as_ref(), path).await
                } else {
                    Err(err)
                }
            }
        }
    }
}

#[derive(Component)]
#[derive_where(Default)]
#[component(storage = "SparseSet")]
pub struct OptionalPayload<A: Asset>(Handle<A>);

pub fn resolve_optional_payload<A: Asset>(
    on_present: impl Fn(&mut EntityCommands, Handle<A>) + Send + Sync + 'static,
    on_absent: impl Fn(&mut EntityCommands) + Send + Sync + 'static,
) -> impl Fn(Commands, Res<AssetServer>, Query<(Entity, &OptionalPayload<A>)>) {
    move |mut commands, asset_server, query| {
        for (entity, OptionalPayload(handle)) in &query {
            let mut entity_commands = commands.entity(entity);
            match asset_server.get_load_state(handle) {
                Some(LoadState::Loaded) => {
                    entity_commands.remove::<OptionalPayload<A>>();
                    on_present(&mut entity_commands, handle.clone());
                }
                Some(LoadState::Failed(_error)) => {
                    entity_commands.remove::<OptionalPayload<A>>();
                    on_absent(&mut entity_commands);
                }
                _ => {}
            }
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PayloadScope {
    root: AssetPath<'static>,
    fonts_root: AssetPath<'static>,
    models_root: AssetPath<'static>,
    textures_root: AssetPath<'static>,
}

impl PayloadScope {
    pub fn new(root: impl Into<AssetPath<'static>>) -> Self {
        let root = root.into();
        Self {
            fonts_root: root.resolve("fonts").unwrap(),
            models_root: root.resolve("models").unwrap(),
            textures_root: root.resolve("textures").unwrap(),
            root,
        }
    }
    fn handle(&self, asset_server: &AssetServer) -> Handle<LoadedFolder> {
        let handle = self.load(asset_server);
        debug_assert!(
            asset_server.is_loaded(&handle),
            "Can't obtain the handle of an unloaded payload scope"
        );
        handle
    }
    fn load(&self, asset_server: &AssetServer) -> Handle<LoadedFolder> {
        asset_server.load_folder(self.root.clone())
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
    pub fn block_texture<'path>(
        &self,
        asset_server: &AssetServer,
        key: &NamespacedKey,
    ) -> (
        Handle<Image>,
        Handle<TextureAtlasLayout>,
        Handle<TextureAtlasAnimation>,
    ) {
        self.animated_texture(
            asset_server,
            &*format!("blocks/{}/{}", key.namespace(), key.key()),
        )
    }
    #[inline]
    pub fn ui_image<'path>(
        &self,
        asset_server: &AssetServer,
        path: impl Into<&'path str>,
    ) -> (
        ImageNode,
        OptionalPayload<TextureAtlasLayout>,
        OptionalPayload<TextureAtlasAnimation>,
    ) {
        let (node, layout, animation) =
            self.image_node(asset_server, &*format!("ui/{}", path.into()));
        (node.with_mode(NodeImageMode::Stretch), layout, animation)
    }
    #[inline]
    pub fn item_image(
        &self,
        asset_server: &AssetServer,
        key: &NamespacedKey,
    ) -> (
        ImageNode,
        OptionalPayload<TextureAtlasLayout>,
        OptionalPayload<TextureAtlasAnimation>,
    ) {
        let (node, layout, animation) = self.image_node(
            asset_server,
            &*format!("items/{}/{}", key.namespace(), key.key()),
        );
        (node.with_mode(NodeImageMode::Stretch), layout, animation)
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
    ) -> (
        ImageNode,
        OptionalPayload<TextureAtlasLayout>,
        OptionalPayload<TextureAtlasAnimation>,
    ) {
        let (image, layout, animation) = self.animated_texture(asset_server, path);
        (
            ImageNode::new(image),
            OptionalPayload(layout),
            OptionalPayload(animation),
        )
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
    fn animated_texture<'path>(
        &self,
        asset_server: &AssetServer,
        path: impl Into<&'path str>,
    ) -> (
        Handle<Image>,
        Handle<TextureAtlasLayout>,
        Handle<TextureAtlasAnimation>,
    ) {
        let path = path.into();
        (
            asset_server.load(
                self.textures_root
                    .resolve(&format!("{}.png", path))
                    .unwrap(),
            ),
            asset_server.load(
                self.textures_root
                    .resolve(&format!("{}.atlas.toml", path))
                    .unwrap(),
            ),
            asset_server.load(
                self.textures_root
                    .resolve(&format!("{}.atlas_animation.toml", path))
                    .unwrap(),
            ),
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
    #[inline]
    pub fn font(&self, asset_server: &AssetServer, key: &NamespacedKey) -> Handle<Font> {
        asset_server.load(
            self.fonts_root
                .resolve(&format!("{}/{}.ttf", key.namespace(), key.key()))
                .unwrap(),
        )
    }
}

pub static GLOBAL_PAYLOADS: LazyLock<PayloadScope> = LazyLock::new(|| PayloadScope::new("global"));

fn load_global_payloads(mut asset_load_requests: MessageWriter<PayloadLoadRequest>) {
    asset_load_requests.write(PayloadLoadRequest {
        scope: &GLOBAL_PAYLOADS,
        parent: None,
    });
}

#[derive(Message)]
pub struct PayloadLoadRequest {
    pub scope: &'static PayloadScope,
    pub parent: Option<&'static PayloadScope>,
}

impl PayloadLoadRequest {
    pub fn new(scope: &'static PayloadScope, parent: &'static PayloadScope) -> Self {
        Self {
            scope,
            parent: Some(parent),
        }
    }
}

#[derive(Message)]
pub struct PayloadLoadedMessage {}

#[derive(Message)]
pub struct PayloadUnloadRequest(pub &'static PayloadScope);

#[derive(Resource, Default)]
pub struct ActivePayloadScopes(
    HashMap<UntypedHandle, (&'static PayloadScope, Option<UntypedHandle>)>, // (scope, parent)
);

pub trait Payloads<A: Asset> {
    fn get_pld(
        &self,
        asset_server: &AssetServer,
        active_scopes: &ActivePayloadScopes,
        handle: &Handle<A>,
    ) -> Option<&A>;
    fn get_pld_mut(
        &mut self,
        asset_server: &AssetServer,
        active_scopes: &ActivePayloadScopes,
        handle: &Handle<A>,
    ) -> Option<&mut A>;
    fn get_pld_mut_untracked(
        &mut self,
        asset_server: &AssetServer,
        active_scopes: &ActivePayloadScopes,
        handle: &Handle<A>,
    ) -> Option<&mut A>;
    fn remove_pld(
        &mut self,
        asset_server: &AssetServer,
        active_scopes: &ActivePayloadScopes,
        handle: &Handle<A>,
    ) -> Option<A>;
    fn remove_pld_untracked(
        &mut self,
        asset_server: &AssetServer,
        active_scopes: &ActivePayloadScopes,
        handle: &Handle<A>,
    ) -> Option<A>;
}

#[inline]
fn resolve_payload_handle<'handle, A: Asset>(
    asset_server: &AssetServer,
    active_scopes: &ActivePayloadScopes,
    asset_handle: &'handle Handle<A>,
) -> Option<Cow<'handle, Handle<A>>> {
    if asset_server.is_loaded(asset_handle) {
        return Some(Cow::Borrowed(asset_handle));
    }
    let asset_path = asset_handle.path()?.path();
    let (mut current_parent, relative_path) =
        active_scopes.0.values().find_map(|(scope, parent)| {
            asset_path
                .strip_prefix(scope.root.path())
                .ok()
                .map(|relative_path| (parent.as_ref(), relative_path.to_string_lossy()))
        })?;
    let relative_path = &*relative_path;
    while let Some(parent) = current_parent {
        let (current_scope, next_parent) = active_scopes.0.get(parent)?;
        let resolved_path = current_scope.root.resolve(relative_path).ok()?;
        if asset_server.get_path_id(&resolved_path).is_some() {
            return Some(Cow::Owned(asset_server.load(resolved_path)));
        }
        current_parent = next_parent.as_ref();
    }
    None
}

impl<A: Asset> Payloads<A> for Assets<A> {
    #[inline]
    fn get_pld(
        &self,
        asset_server: &AssetServer,
        active_scopes: &ActivePayloadScopes,
        handle: &Handle<A>,
    ) -> Option<&A> {
        self.get(&*resolve_payload_handle(
            asset_server,
            active_scopes,
            handle,
        )?)
    }
    #[inline]
    fn get_pld_mut(
        &mut self,
        asset_server: &AssetServer,
        active_scopes: &ActivePayloadScopes,
        handle: &Handle<A>,
    ) -> Option<&mut A> {
        self.get_mut(&*resolve_payload_handle(
            asset_server,
            active_scopes,
            handle,
        )?)
    }
    #[inline]
    fn get_pld_mut_untracked(
        &mut self,
        asset_server: &AssetServer,
        active_scopes: &ActivePayloadScopes,
        handle: &Handle<A>,
    ) -> Option<&mut A> {
        self.get_mut_untracked(&*resolve_payload_handle(
            asset_server,
            active_scopes,
            handle,
        )?)
    }
    #[inline]
    fn remove_pld(
        &mut self,
        asset_server: &AssetServer,
        active_scopes: &ActivePayloadScopes,
        handle: &Handle<A>,
    ) -> Option<A> {
        self.remove(&*resolve_payload_handle(
            asset_server,
            active_scopes,
            handle,
        )?)
    }
    #[inline]
    fn remove_pld_untracked(
        &mut self,
        asset_server: &AssetServer,
        active_scopes: &ActivePayloadScopes,
        handle: &Handle<A>,
    ) -> Option<A> {
        self.remove_untracked(&*resolve_payload_handle(
            asset_server,
            active_scopes,
            handle,
        )?)
    }
}

fn payload_load_request_listener(
    mut requests: MessageReader<PayloadLoadRequest>,
    asset_server: Res<AssetServer>,
    mut loaded_payloads: ResMut<ActivePayloadScopes>,
    mut game_state: ResMut<NextState<GameState>>,
) {
    for request in requests.read() {
        loaded_payloads
            .0
            .insert(request.scope.load(&asset_server).untyped(), {
                let PayloadLoadRequest { scope, parent } = request;
                (
                    scope,
                    parent.map(|parent_scope| parent_scope.handle(&asset_server).untyped()),
                )
            });
        game_state.set(GameState::Loading);
    }
}

fn payload_unload_request_listener(
    mut requests: MessageReader<PayloadUnloadRequest>,
    asset_server: Res<AssetServer>,
    mut loaded_payloads: ResMut<ActivePayloadScopes>,
) {
    for request in requests.read() {
        asset_server
            .get_path_id(request.0.root.clone())
            .and_then(|untyped_id| asset_server.get_id_handle_untyped(untyped_id))
            .map(|handle| loaded_payloads.0.remove(&handle));
    }
}

fn folder_loaded_listener(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut folder_events: MessageReader<AssetEvent<LoadedFolder>>,
    mut messages: MessageWriter<PayloadLoadedMessage>,
    loaded_payloads: Res<ActivePayloadScopes>,
) {
    for event in folder_events.read() {
        if let AssetEvent::LoadedWithDependencies { id } = event {
            messages.write(PayloadLoadedMessage {});
            commands.trigger(ReloadMetadata {
                scope: loaded_payloads.0[&asset_server.get_id_handle(*id).unwrap().untyped()].0,
            });
        }
    }
}

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<ActivePayloadScopes>()
        .add_message::<PayloadLoadRequest>()
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
        .add_systems(Update, folder_loaded_listener)
        .add_plugins(meta::plugin);
}
