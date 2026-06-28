//! *Payload*(pld)*s* are resources that the game uses during execution, such as assets or game data.
//! *Shipment*(shp)*s* are their processed counterpart.

pub mod meta;

use crate::path;
use crate::ui::{TextureAnimation, TextureScaling};
use crate::utils::NamespacedKey;
use bevy::app::App;
use bevy::asset::io::AssetSourceId;
use bevy::asset::{AssetPath, LoadState, LoadedFolder, embedded_asset};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use derive_where::derive_where;
use std::cmp::PartialEq;
use std::collections::HashSet;
use std::marker::PhantomData;
use uuid::Uuid;

#[derive(Component)]
#[derive_where(Default)]
#[component(storage = "SparseSet")]
pub struct OptionalPayload<A: Asset>(Handle<A>);

pub fn resolve_optional_payload<A: Asset>(
    // TODO use bsn after Bevy 0.19
    on_present: impl Fn(&mut EntityCommands, Handle<A>, &A) + Send + Sync + 'static,
    on_absent: impl Fn(&mut EntityCommands) + Send + Sync + 'static,
) -> impl Fn(Commands, Res<AssetServer>, Res<Assets<A>>, Query<(Entity, &OptionalPayload<A>)>) {
    move |mut commands, asset_server, payloads, query| {
        for (entity, OptionalPayload(handle)) in &query {
            let mut entity_commands = commands.entity(entity);
            match asset_server.get_load_state(handle) {
                Some(LoadState::Loaded) => {
                    entity_commands.remove::<OptionalPayload<A>>();
                    on_present(
                        &mut entity_commands,
                        handle.clone(),
                        payloads.get(handle).unwrap(),
                    );
                }
                Some(LoadState::Failed(..)) | None => {
                    entity_commands.remove::<OptionalPayload<A>>();
                    on_absent(&mut entity_commands);
                }
                _ => {}
            }
        }
    }
}

#[derive(Default, Resource)]
struct PayloadHold {
    loading_scopes: HashSet<Handle<LoadedFolder>>,
    loaded_scopes: HashSet<Handle<LoadedFolder>>,
}

#[derive(Event)]
pub struct PayloadFetchingComplete;

fn monitor_folder_loads(
    mut folder_events_reader: MessageReader<AssetEvent<LoadedFolder>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut payload_hold: ResMut<PayloadHold>,
) {
    for folder_event in folder_events_reader.read() {
        if let AssetEvent::LoadedWithDependencies { id } = folder_event {
            let handle = asset_server.get_id_handle(*id).unwrap();
            if payload_hold.loading_scopes.remove(&handle) {
                payload_hold.loaded_scopes.insert(handle);
                if payload_hold.loading_scopes.is_empty() {
                    commands.trigger(PayloadFetchingComplete);
                }
            }
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum PayloadScopeId {
    Global,
    Dimension(NamespacedKey),
}

impl PayloadScopeId {
    fn build<'src_id>(&self, source: impl Into<AssetSourceId<'src_id>>) -> PayloadScope {
        match self {
            Self::Global => PayloadScope::new(source, "global"),
            Self::Dimension(key) => PayloadScope::new(source, format!("dim/{}", key.path_string())),
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
    pub fn new<'src_id, 'path>(
        source: impl Into<AssetSourceId<'src_id>>,
        root: impl Into<AssetPath<'path>>,
    ) -> Self {
        let root = root
            .into()
            .into_owned()
            .with_source(source.into().into_owned());
        Self {
            fonts_root: root.resolve("fonts").unwrap(),
            models_root: root.resolve("models").unwrap(),
            textures_root: root.resolve("textures").unwrap(),
            root,
        }
    }
}

#[derive(Resource)]
pub struct PayloadManager {
    scope_ids: Vec<PayloadScopeId>,
    sources: Vec<(Vec<PayloadScope>, AssetSourceId<'static>)>, // TODO optimize memory layout
}

impl PayloadManager {
    fn new() -> Self {
        Self {
            scope_ids: Vec::with_capacity(2),
            sources: Vec::with_capacity(1),
        }
    }
}

// TODO do we need this?
#[derive(SystemParam)]
pub struct PayloadResolutionParam<'w, 's, A: Asset> {
    payload_manager: Res<'w, PayloadManager>,
    asset_server: Res<'w, AssetServer>,
    assets: Res<'w, Assets<A>>,
    _marker: PhantomData<&'s ()>,
}

#[derive(Event)]
pub struct FetchPayloadScopeRequest(PayloadScopeId);

impl FetchPayloadScopeRequest {
    pub fn new(scope_id: PayloadScopeId) -> Self {
        Self(scope_id)
    }
}

fn handle_fetch_scope_request(
    request: On<FetchPayloadScopeRequest>,
    asset_server: Res<AssetServer>,
    mut payload_manager: ResMut<PayloadManager>,
    mut payload_hold: ResMut<PayloadHold>,
) {
    let FetchPayloadScopeRequest(scope_id) = &*request;
    payload_manager.scope_ids.push(scope_id.clone());
    for (scopes, source_id) in &mut payload_manager.sources {
        let scope = scope_id.build(source_id.clone());
        payload_hold
            .loading_scopes
            .insert(asset_server.load_folder(&scope.root));
        println!("Load {}", scope.root);
        scopes.push(scope);
    }
}

#[derive(Event)]
pub struct EvictPayloadScopeRequest(PayloadScopeId);

impl EvictPayloadScopeRequest {
    pub fn new(scope_id: PayloadScopeId) -> Self {
        Self(scope_id)
    }
}

fn handle_evict_scope_request(
    request: On<EvictPayloadScopeRequest>,
    asset_server: Res<AssetServer>,
    mut payload_manager: ResMut<PayloadManager>,
    mut payload_hold: ResMut<PayloadHold>,
) {
    let EvictPayloadScopeRequest(scope_id) = &*request;
    assert!(
        payload_manager
            .scope_ids
            .last()
            .is_some_and(|id| id == scope_id),
        "Only the topmost scope may be evicted."
    );
    payload_manager.scope_ids.pop();
    for (scopes, _source_id) in &mut payload_manager.sources {
        payload_hold.loaded_scopes.remove(
            &asset_server
                .get_handle(&scopes.pop().unwrap().root)
                .unwrap(),
        );
    }
}

#[derive(Event)]
pub struct MountPayloadSourceRequest(AssetSourceId<'static>);

impl MountPayloadSourceRequest {
    pub fn new<'src_id>(source_id: impl Into<AssetSourceId<'src_id>>) -> Self {
        Self(source_id.into().into_owned())
    }
}

fn handle_mount_source_request(
    request: On<MountPayloadSourceRequest>,
    asset_server: Res<AssetServer>,
    mut payload_manager: ResMut<PayloadManager>,
    mut payload_hold: ResMut<PayloadHold>,
) {
    let MountPayloadSourceRequest(source_id) = &*request;
    let PayloadManager { scope_ids, sources } = &mut *payload_manager;
    sources.push((
        scope_ids
            .iter()
            .map(|scope_id| scope_id.build(source_id.clone()))
            .inspect(|scope| {
                payload_hold
                    .loading_scopes
                    .insert(asset_server.load_folder(&scope.root));
            })
            .collect(),
        source_id.clone(),
    ));
}

#[derive(Event)]
pub struct UnmountPayloadSourceRequest(AssetSourceId<'static>);

impl UnmountPayloadSourceRequest {
    pub fn new<'src_id>(source_id: impl Into<AssetSourceId<'src_id>>) -> Self {
        Self(source_id.into().into_owned())
    }
}

fn handle_unmount_source_request(
    request: On<UnmountPayloadSourceRequest>,
    asset_server: Res<AssetServer>,
    mut payload_manager: ResMut<PayloadManager>,
    mut payload_hold: ResMut<PayloadHold>,
) {
    let UnmountPayloadSourceRequest(source_id) = &*request;
    match payload_manager
        .sources
        .iter()
        .position(|(_scopes, id)| source_id == id)
    {
        Some(index) => {
            let (scopes, _source_id) = payload_manager.sources.remove(index);
            for scope in scopes {
                payload_hold
                    .loaded_scopes
                    .remove(&asset_server.get_handle(scope.root).unwrap());
            }
        }
        None => error!(
            "The specified source could not be unloaded because it is not loaded: {}",
            source_id
        ),
    }
}

#[derive(Event)]
pub struct RefetchPayloadRequest;

fn handle_reload_request(
    request: On<RefetchPayloadRequest>,
    asset_server: Res<AssetServer>,
    payload_manager: Res<PayloadManager>,
    mut payload_hold: ResMut<PayloadHold>,
) {
    let RefetchPayloadRequest = &*request;
    let PayloadHold {
        loading_scopes,
        loaded_scopes,
    } = &mut *payload_hold;
    loading_scopes.extend(loaded_scopes.drain());
    for source in &payload_manager.sources {
        for scope in &source.0 {
            asset_server.reload(scope.root.clone());
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum GltfElementId<'name> {
    #[default]
    Default,
    Index(usize),
    Name(&'name str),
}

fn resolve<'path, A: Asset>(
    payload_manager: &PayloadManager,
    asset_server: &AssetServer,
    assets: &Assets<A>,
    path: impl Into<AssetPath<'path>>,
) -> Option<Handle<A>> {
    let path = path.into();
    for (scopes, _source_id) in payload_manager.sources.iter().rev() {
        for scope in scopes.iter().rev() {
            if let Some(handle) = asset_server
                .get_handle(scope.root.resolve(&*path.path().to_string_lossy()).unwrap())
            {
                return Some(handle);
            }
        }
    }
    let uuid = Uuid::new_v5(
        &Uuid::new_v5(&Uuid::NAMESPACE_URL, A::type_path().as_bytes()),
        path.to_string().as_bytes(),
    );
    if assets.contains(uuid.clone()) {
        return Some(Handle::Uuid(uuid, PhantomData));
    }
    None
}
#[inline]
pub fn actor_scene(
    payload_manager: &PayloadManager,
    asset_server: &AssetServer,
    models: &Assets<Gltf>,
    key: &NamespacedKey,
    id: GltfElementId,
) -> Option<Handle<Scene>> {
    scene(
        payload_manager,
        asset_server,
        models,
        &*format!("actors/{}", key.path_string()),
        id,
    )
}
#[inline]
pub fn animate_actor(
    payload_manager: &PayloadManager,
    animation_player: &mut AnimationPlayer,
    asset_server: &AssetServer,
    models: &Assets<Gltf>,
    key: &NamespacedKey,
    id: GltfElementId,
) -> Option<AnimationGraphHandle> {
    animate(
        payload_manager,
        animation_player,
        asset_server,
        models,
        &*format!("actors/{}", key.path_string()),
        id,
    )
}
#[inline]
pub fn block_texture<'path>(
    payload_manager: &PayloadManager,
    asset_server: &AssetServer,
    images: &Assets<Image>,
    texture_atlas_layouts: &Assets<TextureAtlasLayout>,
    texture_animations: &Assets<TextureAnimation>,
    texture_scalings: &Assets<TextureScaling>,
    key: &NamespacedKey,
) -> (
    Handle<Image>,
    Option<Handle<TextureAtlasLayout>>,
    Option<Handle<TextureAnimation>>,
    Option<Handle<TextureScaling>>,
) {
    rich_image(
        payload_manager,
        asset_server,
        images,
        texture_atlas_layouts,
        texture_animations,
        texture_scalings,
        &*format!("blocks/{}", key.path_string()),
    )
}
#[inline]
pub fn ui_image_node<'path>(
    payload_manager: &PayloadManager,
    asset_server: &AssetServer,
    images: &Assets<Image>,
    texture_atlas_layouts: &Assets<TextureAtlasLayout>,
    texture_animations: &Assets<TextureAnimation>,
    texture_scalings: &Assets<TextureScaling>,
    path: impl Into<&'path str>,
) -> (
    ImageNode,
    OptionalPayload<TextureAnimation>, // TODO use bsn after Bevy 0.19
) {
    image_node(
        payload_manager,
        asset_server,
        images,
        texture_atlas_layouts,
        texture_animations,
        texture_scalings,
        &*format!("ui/{}", path.into()),
    )
}
#[inline]
pub fn item_image_node(
    payload_manager: &PayloadManager,
    asset_server: &AssetServer,
    images: &Assets<Image>,
    texture_atlas_layouts: &Assets<TextureAtlasLayout>,
    texture_animations: &Assets<TextureAnimation>,
    texture_scalings: &Assets<TextureScaling>,
    key: &NamespacedKey,
) -> (
    ImageNode,
    OptionalPayload<TextureAnimation>, // TODO use bsn after Bevy 0.19
) {
    image_node(
        payload_manager,
        asset_server,
        images,
        texture_atlas_layouts,
        texture_animations,
        texture_scalings,
        &*format!("items/{}", key.path_string()),
    )
}
#[inline]
pub fn default_scene(
    payload_manager: &PayloadManager,
    asset_server: &AssetServer,
    models: &Assets<Gltf>,
) -> Handle<Scene> {
    // TODO use embedded bsn when bsn reader comes out
    scene(
        payload_manager,
        asset_server,
        models,
        "missingno",
        GltfElementId::Default,
    )
    .unwrap()
}
#[inline]
fn image_node<'path>(
    payload_manager: &PayloadManager,
    asset_server: &AssetServer,
    images: &Assets<Image>,
    texture_atlas_layouts: &Assets<TextureAtlasLayout>,
    texture_animations: &Assets<TextureAnimation>,
    texture_scalings: &Assets<TextureScaling>,
    path: impl Into<&'path str>,
) -> (
    ImageNode,
    OptionalPayload<TextureAnimation>, // TODO use bsn after Bevy 0.19
) {
    let (image, atlas, animation, scaling) = rich_image(
        payload_manager,
        asset_server,
        images,
        texture_atlas_layouts,
        texture_animations,
        texture_scalings,
        path,
    );
    let mut node = ImageNode::new(image).with_mode(
        scaling
            .and_then(|scaling| texture_scalings.get(&scaling))
            .map(NodeImageMode::from)
            .unwrap_or(NodeImageMode::Stretch),
    );
    node.texture_atlas = atlas.map(TextureAtlas::from);
    (node, OptionalPayload(animation.unwrap_or_default()))
}
#[inline]
fn scene<'path>(
    payload_manager: &PayloadManager,
    asset_server: &AssetServer,
    models: &Assets<Gltf>,
    path: impl Into<&'path str>,
    id: GltfElementId,
) -> Option<Handle<Scene>> {
    model(payload_manager, asset_server, models, path)
        .and_then(|handle| models.get(&handle))
        .and_then(|gltf| match id {
            GltfElementId::Default => gltf.default_scene.as_ref(),
            GltfElementId::Index(index) => gltf.scenes.get(index),
            GltfElementId::Name(name) => gltf.named_scenes.get(name),
        })
        .cloned()
}
#[inline]
fn animate<'path>(
    payload_manager: &PayloadManager,
    animation_player: &mut AnimationPlayer,
    asset_server: &AssetServer,
    models: &Assets<Gltf>,
    path: impl Into<&'path str>,
    id: GltfElementId,
) -> Option<AnimationGraphHandle> {
    animation(payload_manager, asset_server, models, path, id)
        .map(AnimationGraph::from_clip)
        .map(|(graph, index)| {
            animation_player.play(index).repeat();
            AnimationGraphHandle(asset_server.add(graph))
        })
}
#[inline]
fn animation<'path>(
    payload_manager: &PayloadManager,
    asset_server: &AssetServer,
    models: &Assets<Gltf>,
    path: impl Into<&'path str>,
    id: GltfElementId,
) -> Option<Handle<AnimationClip>> {
    model(payload_manager, asset_server, models, path)
        .and_then(|handle| models.get(&handle))
        .and_then(|gltf| match id {
            GltfElementId::Default => None,
            GltfElementId::Index(index) => gltf.animations.get(index),
            GltfElementId::Name(name) => gltf.named_animations.get(name),
        })
        .cloned()
}
#[inline]
fn plain_image<'path>(
    payload_manager: &PayloadManager,
    asset_server: &AssetServer,
    images: &Assets<Image>,
    path: impl Into<&'path str>,
) -> Handle<Image> {
    resolve(
        payload_manager,
        asset_server,
        images,
        format!("textures/{}.png", path.into()),
    )
    .unwrap_or_else(|| missingno(asset_server))
}
#[inline]
fn rich_image<'path>(
    payload_manager: &PayloadManager,
    asset_server: &AssetServer,
    images: &Assets<Image>,
    texture_atlas_layouts: &Assets<TextureAtlasLayout>,
    texture_animations: &Assets<TextureAnimation>,
    texture_scalings: &Assets<TextureScaling>,
    path: impl Into<&'path str>,
) -> (
    Handle<Image>,
    Option<Handle<TextureAtlasLayout>>,
    Option<Handle<TextureAnimation>>,
    Option<Handle<TextureScaling>>,
) {
    let path = path.into();
    (
        resolve(
            payload_manager,
            asset_server,
            images,
            format!("textures/{}.png", path),
        )
        .unwrap_or_else(|| missingno(asset_server)),
        resolve(
            payload_manager,
            asset_server,
            texture_atlas_layouts,
            format!("textures/{}.atlas.toml", path),
        ),
        resolve(
            payload_manager,
            asset_server,
            texture_animations,
            format!("textures/{}.animation.toml", path),
        ),
        resolve(
            payload_manager,
            asset_server,
            texture_scalings,
            format!("textures/{}.scaling.toml", path),
        ),
    )
}
#[inline]
fn model<'path>(
    payload_manager: &PayloadManager,
    asset_server: &AssetServer,
    models: &Assets<Gltf>,
    path: impl Into<&'path str>,
) -> Option<Handle<Gltf>> {
    resolve(
        payload_manager,
        asset_server,
        models,
        format!("models/{}.glb", path.into()),
    )
}
#[inline]
pub fn font(
    payload_manager: &PayloadManager,
    asset_server: &AssetServer,
    fonts: &Assets<Font>,
    key: &NamespacedKey,
) -> Option<Handle<Font>> {
    resolve(
        payload_manager,
        asset_server,
        fonts,
        format!("fonts/{}.ttf", key.path_string()),
    )
}
#[inline]
pub fn missingno(asset_server: &AssetServer) -> Handle<Image> {
    asset_server.load("embedded://embers/missingno.png")
}

pub(super) fn plugin(app: &mut App) {
    embedded_asset!(app, path!("icon.png"));
    embedded_asset!(app, path!("missingno.png"));
    app.init_resource::<PayloadHold>()
        .insert_resource(PayloadManager::new())
        .add_systems(Update, monitor_folder_loads)
        .add_observer(handle_fetch_scope_request)
        .add_observer(handle_evict_scope_request)
        .add_observer(handle_mount_source_request)
        .add_observer(handle_unmount_source_request)
        .add_observer(handle_reload_request)
        .add_plugins(meta::plugin);
}
