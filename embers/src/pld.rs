//! *Payload*(pld)*s* are resources that the game uses during execution, such as assets or game data.
//! *Shipment*(shp)*s* are their processed counterpart.

pub mod meta;

use crate::path;
use crate::ui::{AnimatedTexture, TextureAnimation, TextureScaling};
use crate::utils::{Keyed, NamespacedKey, RemoveComponentTemplate};
use atomicow::CowArc;
use bevy::app::App;
use bevy::asset::io::AssetSourceId;
use bevy::asset::{AssetPath, LoadedFolder, embedded_asset};
use bevy::ecs::template::{TemplateContext, TemplateTuple};
use bevy::gltf::{GltfMaterial, GltfMesh, GltfNode, GltfSkin};
use bevy::prelude::*;
use derive_where::derive_where;
use std::any::TypeId;
use std::cmp::PartialEq;
use std::collections::HashSet;
use std::ffi::OsStr;
use std::marker::PhantomData;
use std::path::Path;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use uuid::Uuid;

pub trait PayloadPath {
    fn with_added_extension<S: AsRef<OsStr>>(&self, extension: S) -> Self;
}

impl PayloadPath for AssetPath<'_> {
    fn with_added_extension<S: AsRef<OsStr>>(&self, extension: S) -> Self {
        let mut path = self.path().to_path_buf();
        path.add_extension(extension);
        let path = AssetPath::from_path_buf(path).with_source(self.source().clone_owned());
        match self.label_cow() {
            Some(label) => path.with_label(label.into_owned()),
            None => path,
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
            fonts_root: root.resolve_str("fonts").unwrap(),
            models_root: root.resolve_str("models").unwrap(),
            textures_root: root.resolve_str("textures").unwrap(),
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

fn resolve<'path, A: Asset>(
    payload_manager: &PayloadManager,
    asset_server: &AssetServer,
    assets: &Assets<A>,
    path: impl Into<AssetPath<'path>>,
) -> Option<Handle<A>> {
    let path = path.into();
    for (scopes, _source_id) in payload_manager.sources.iter().rev() {
        for scope in scopes.iter().rev() {
            if let Some(handle) = asset_server.get_handle(scope.root.resolve(&path)) {
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
pub fn pld<A: Asset>(value: impl Into<PayloadTemplate<A>>) -> PayloadTemplate<A> {
    value.into()
}

#[inline]
pub fn optional_pld<A: Asset>(value: impl Into<PayloadTemplate<A>>) -> OptionalPayloadTemplate<A> {
    OptionalPayloadTemplate(pld(value))
}

#[inline]
pub fn payload_value<A: Asset>(value: impl Into<A>) -> PayloadTemplate<A> {
    PayloadTemplate::value(value.into())
}

#[derive(Debug)]
pub enum PayloadTemplate<A: Asset> {
    Handle(Handle<A>),
    Path(AssetPath<'static>),
    Value(Arc<Mutex<Result<Option<A>, Handle<A>>>>),
}

impl<A: Asset> Default for PayloadTemplate<A> {
    fn default() -> Self {
        Self::Handle(default())
    }
}

impl<A: Asset> From<Handle<A>> for PayloadTemplate<A> {
    fn from(value: Handle<A>) -> Self {
        Self::Handle(value)
    }
}

impl<A: Asset> From<AssetPath<'_>> for PayloadTemplate<A> {
    fn from(value: AssetPath<'_>) -> Self {
        Self::Path(value.into_owned())
    }
}

impl<A: Asset> PayloadTemplate<A> {
    pub fn value(value: A) -> Self {
        Self::Value(Arc::new(Mutex::new(Ok(Some(value)))))
    }
}

#[derive(Debug, Error)]
#[error("Couldn't find payload in {path}")]
struct PayloadNotFoundError {
    path: AssetPath<'static>,
}

impl<A: Asset> Template for PayloadTemplate<A> {
    type Output = Handle<A>;
    fn build_template(&self, context: &mut TemplateContext) -> Result<Self::Output> {
        match self {
            Self::Handle(handle) => Ok(handle.clone()),
            Self::Path(path) => resolve(
                context.entity.resource::<PayloadManager>(),
                context.entity.resource::<AssetServer>(),
                context.entity.resource::<Assets<A>>(),
                path,
            )
            .or_else(|| fallback(context.entity.resource::<AssetServer>()))
            .ok_or_else(|| BevyError::error(PayloadNotFoundError { path: path.clone() })),
            Self::Value(value_or_handle) => {
                let value_or_handle = &mut *value_or_handle.lock().unwrap();
                match value_or_handle {
                    Ok(value) => {
                        let handle = context
                            .resource_mut::<Assets<A>>()
                            .add(value.take().unwrap());
                        *value_or_handle = Err(handle.clone());
                        Ok(handle)
                    }
                    Err(handle) => Ok(handle.clone()),
                }
            }
        }
    }
    fn clone_template(&self) -> Self {
        match self {
            Self::Handle(handle) => Self::Handle(handle.clone()),
            Self::Path(path) => Self::Path(path.clone()),
            Self::Value(value) => Self::Value(value.clone()),
        }
    }
}

#[derive_where(Default)]
pub struct OptionalPayloadTemplate<A: Asset>(pub PayloadTemplate<A>);

impl<A: Asset> Template for OptionalPayloadTemplate<A> {
    type Output = Option<Handle<A>>;
    fn build_template(&self, context: &mut TemplateContext) -> Result<Self::Output> {
        Ok(self.0.build_template(context).ok())
    }
    fn clone_template(&self) -> Self {
        Self(self.0.clone_template())
    }
}

#[derive(Default)]
struct TextFontTemplate {
    font: PayloadTemplate<Font>,
    size: FontSize,
}

impl Template for TextFontTemplate {
    type Output = TextFont;
    fn build_template(&self, context: &mut TemplateContext) -> Result<Self::Output> {
        Ok(TextFont {
            font: FontSource::Handle(self.font.build_template(context)?),
            font_size: self.size,
            font_smoothing: FontSmoothing::None,
            ..default()
        })
    }
    fn clone_template(&self) -> Self {
        Self {
            font: self.font.clone_template(),
            size: self.size,
        }
    }
}

#[inline]
pub fn text_font(key: &impl Keyed, size: impl Into<FontSize>) -> impl Scene {
    template_value(TextFontTemplate {
        font: pld(AssetPath::from_path(Path::new("fonts"))
            .resolve_str(&*key.key().path_string())
            .expect("Invalid font key")
            .with_added_extension("ttf")),
        size: size.into(),
    })
}

#[inline]
fn model<'path>(path: impl Into<AssetPath<'path>>) -> PayloadTemplate<Gltf> {
    pld(AssetPath::from_path(Path::new("models"))
        .resolve(&path.into())
        .with_added_extension("glb"))
}

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub enum GltfElementId<'name> {
    #[default]
    Default,
    Index(usize),
    Name(CowArc<'name, str>),
}

impl GltfElementId<'_> {
    pub fn into_owned(self) -> GltfElementId<'static> {
        match self {
            Self::Default => GltfElementId::Default,
            Self::Index(index) => GltfElementId::Index(index),
            Self::Name(name) => GltfElementId::Name(name.into_owned()),
        }
    }
    pub fn clone_owned(&self) -> GltfElementId<'static> {
        self.clone().into_owned()
    }
}

pub trait GltfElement: Asset {
    fn from_gltf<'gltf>(gltf: &'gltf Gltf, id: &GltfElementId) -> Option<&'gltf Handle<Self>>
    where
        Self: Sized;
}

impl GltfElement for WorldAsset {
    fn from_gltf<'gltf>(gltf: &'gltf Gltf, id: &GltfElementId) -> Option<&'gltf Handle<Self>>
    where
        Self: Sized,
    {
        match id {
            GltfElementId::Default => gltf.default_scene.as_ref(),
            GltfElementId::Index(index) => gltf.scenes.get(*index),
            GltfElementId::Name(name) => gltf.named_scenes.get(name.as_ref()),
        }
    }
}

impl GltfElement for GltfMesh {
    fn from_gltf<'gltf>(gltf: &'gltf Gltf, id: &GltfElementId) -> Option<&'gltf Handle<Self>>
    where
        Self: Sized,
    {
        match id {
            GltfElementId::Default => None,
            GltfElementId::Index(index) => gltf.meshes.get(*index),
            GltfElementId::Name(name) => gltf.named_meshes.get(name.as_ref()),
        }
    }
}

impl GltfElement for GltfMaterial {
    fn from_gltf<'gltf>(gltf: &'gltf Gltf, id: &GltfElementId) -> Option<&'gltf Handle<Self>>
    where
        Self: Sized,
    {
        match id {
            GltfElementId::Default => None,
            GltfElementId::Index(index) => gltf.materials.get(*index),
            GltfElementId::Name(name) => gltf.named_materials.get(name.as_ref()),
        }
    }
}

impl GltfElement for GltfNode {
    fn from_gltf<'gltf>(gltf: &'gltf Gltf, id: &GltfElementId) -> Option<&'gltf Handle<Self>>
    where
        Self: Sized,
    {
        match id {
            GltfElementId::Default => None,
            GltfElementId::Index(index) => gltf.nodes.get(*index),
            GltfElementId::Name(name) => gltf.named_nodes.get(name.as_ref()),
        }
    }
}

impl GltfElement for GltfSkin {
    fn from_gltf<'gltf>(gltf: &'gltf Gltf, id: &GltfElementId) -> Option<&'gltf Handle<Self>>
    where
        Self: Sized,
    {
        match id {
            GltfElementId::Default => None,
            GltfElementId::Index(index) => gltf.skins.get(*index),
            GltfElementId::Name(name) => gltf.named_skins.get(name.as_ref()),
        }
    }
}

impl GltfElement for AnimationClip {
    fn from_gltf<'gltf>(gltf: &'gltf Gltf, id: &GltfElementId) -> Option<&'gltf Handle<Self>>
    where
        Self: Sized,
    {
        match id {
            GltfElementId::Default => None,
            GltfElementId::Index(index) => gltf.animations.get(*index),
            GltfElementId::Name(name) => gltf.named_animations.get(name.as_ref()),
        }
    }
}

#[derive_where(Default)]
pub struct GltfElementTemplate<E: GltfElement> {
    model: PayloadTemplate<Gltf>,
    id: GltfElementId<'static>,
    _marker: PhantomData<E>,
}

#[derive(Debug, Error)]
#[error("Couldn't find gltf element {id:?} in {model:?}")]
struct GltfElementNotFoundError {
    model: PayloadTemplate<Gltf>,
    id: GltfElementId<'static>,
}

impl<E: GltfElement> Template for GltfElementTemplate<E> {
    type Output = Handle<E>;
    fn build_template(&self, context: &mut TemplateContext) -> Result<Self::Output> {
        self.model
            .build_template(context)
            .map(|handle| context.resource::<Assets<Gltf>>().get(&handle).unwrap())
            .and_then(|gltf| {
                E::from_gltf(&gltf, &self.id).cloned().ok_or_else(|| {
                    BevyError::error(GltfElementNotFoundError {
                        model: self.model.clone_template(),
                        id: self.id.clone(),
                    })
                })
            })
    }
    fn clone_template(&self) -> Self {
        Self {
            model: self.model.clone_template(),
            id: self.id.clone(),
            _marker: PhantomData,
        }
    }
}

#[inline]
fn model_element<'path, E: GltfElement>(
    path: impl Into<AssetPath<'path>>,
    id: GltfElementId,
) -> GltfElementTemplate<E> {
    GltfElementTemplate {
        model: model(path),
        id: id.into_owned(),
        _marker: PhantomData,
    }
}

pub struct AnimationGraphHandleTemplate {
    clip: GltfElementTemplate<AnimationClip>,
}

impl Template for AnimationGraphHandleTemplate {
    type Output = AnimationGraphHandle;
    fn build_template(&self, context: &mut TemplateContext) -> Result<Self::Output> {
        self.clip
            .build_template(context)
            .map(AnimationGraph::from_clip)
            .map(|(graph, index)| {
                context
                    .entity
                    .insert_if_new(AnimationPlayer::default())
                    .get_mut::<AnimationPlayer>()
                    .unwrap()
                    .play(index)
                    .repeat();
                AnimationGraphHandle(context.resource_mut::<Assets<AnimationGraph>>().add(graph))
            })
    }
    fn clone_template(&self) -> Self {
        Self {
            clip: self.clip.clone_template(),
        }
    }
}

#[inline]
fn animate<'path>(
    path: impl Into<AssetPath<'path>>,
    id: GltfElementId,
) -> AnimationGraphHandleTemplate {
    AnimationGraphHandleTemplate {
        clip: model_element(path, id),
    }
}

#[inline]
pub fn animate_actor(key: &NamespacedKey, id: GltfElementId) -> AnimationGraphHandleTemplate {
    animate(format!("actors/{}", key.path_string()), id)
}

#[derive(Default)]
struct WorldAssetRootTemplate {
    scene: GltfElementTemplate<WorldAsset>,
}

impl Template for WorldAssetRootTemplate {
    type Output = WorldAssetRoot;
    fn build_template(&self, context: &mut TemplateContext) -> Result<Self::Output> {
        Ok(WorldAssetRoot(self.scene.build_template(context)?))
    }
    fn clone_template(&self) -> Self {
        Self {
            scene: self.scene.clone_template(),
        }
    }
}

#[inline]
pub fn default_scene() -> GltfElementTemplate<WorldAsset> {
    // TODO use embedded bsn when bsn reader comes out
    model_element("missingno", GltfElementId::Default)
}

#[inline]
pub fn actor_scene(key: &NamespacedKey, id: GltfElementId) -> WorldAssetRootTemplate {
    WorldAssetRootTemplate {
        scene: model_element(format!("actors/{}", key.path_string()), id),
    }
}

#[inline]
fn plain_image<'path>(path: impl Into<AssetPath<'path>>) -> PayloadTemplate<Image> {
    pld(AssetPath::from_path(Path::new("textures"))
        .resolve(&path.into())
        .with_added_extension("png"))
}

#[inline]
fn rich_image<'path>(
    path: impl Into<AssetPath<'path>>,
) -> TemplateTuple<(
    PayloadTemplate<Image>,
    OptionalPayloadTemplate<TextureAtlasLayout>,
    OptionalPayloadTemplate<TextureAnimation>,
    OptionalPayloadTemplate<TextureScaling>,
)> {
    let path = AssetPath::from_path(Path::new("textures")).resolve(&path.into());
    TemplateTuple((
        pld(path.with_added_extension("png")),
        optional_pld(path.with_added_extension("atlas.toml")),
        optional_pld(path.with_added_extension("animation.toml")),
        optional_pld(path.with_added_extension("scaling.toml")),
    ))
}

#[inline]
pub fn block_texture(
    key: &NamespacedKey,
) -> TemplateTuple<(
    PayloadTemplate<Image>,
    OptionalPayloadTemplate<TextureAtlasLayout>,
    OptionalPayloadTemplate<TextureAnimation>,
    OptionalPayloadTemplate<TextureScaling>,
)> {
    rich_image(format!("blocks/{}", key.path_string()))
}

#[derive(Default)]
struct ImageNodeTemplate {
    image: PayloadTemplate<Image>,
    atlas: OptionalPayloadTemplate<TextureAtlasLayout>,
    scaling: OptionalPayloadTemplate<TextureScaling>,
}

impl Template for ImageNodeTemplate {
    type Output = ImageNode;
    fn build_template(&self, context: &mut TemplateContext) -> Result<Self::Output> {
        let mut node = ImageNode::new(self.image.build_template(context)?).with_mode(
            self.scaling
                .build_template(context)?
                .map(|scaling| {
                    context
                        .resource::<Assets<TextureScaling>>()
                        .get(&scaling)
                        .unwrap()
                })
                .map(NodeImageMode::from)
                .unwrap_or(NodeImageMode::Stretch),
        );
        node.texture_atlas = self.atlas.build_template(context)?.map(TextureAtlas::from);
        Ok(node)
    }
    fn clone_template(&self) -> Self {
        Self {
            image: self.image.clone_template(),
            atlas: self.atlas.clone_template(),
            scaling: self.scaling.clone_template(),
        }
    }
}

#[derive(Default)]
struct TextureAnimationTemplate {
    animation: OptionalPayloadTemplate<TextureAnimation>,
}

#[derive(Debug, Error)]
#[error("Texture animation was not found")]
struct TextureAnimationNotFoundError;

impl Template for TextureAnimationTemplate {
    type Output = AnimatedTexture;
    fn build_template(&self, context: &mut TemplateContext) -> Result<Self::Output> {
        self.animation
            .build_template(context)
            .and_then(|animation| match animation {
                Some(animation) => Ok(AnimatedTexture::new(
                    context
                        .resource::<Assets<TextureAnimation>>()
                        .get(&animation)
                        .unwrap()
                        .clone(),
                )),
                None => Err(BevyError::ignore(TextureAnimationNotFoundError)),
            })
            .inspect_err(|_error| {
                context.entity.remove::<AnimatedTexture>();
            })
    }
    fn clone_template(&self) -> Self {
        Self {
            animation: self.animation.clone_template(),
        }
    }
}

#[inline]
fn image_node<'path>(path: impl Into<AssetPath<'path>>) -> impl Scene {
    let TemplateTuple((image, atlas, animation, scaling)) = rich_image(path);
    bsn! {
        template_value(ImageNodeTemplate { image, atlas, scaling })
        template_value(TextureAnimationTemplate { animation })
    }
}

#[inline]
pub fn empty_image_node() -> impl Scene {
    bsn! {
        template_value(RemoveComponentTemplate::<ImageNode>::new())
        template_value(RemoveComponentTemplate::<AnimatedTexture>::new())
    }
}

#[inline]
pub fn ui_image_node<'path>(path: impl Into<AssetPath<'path>>) -> impl Scene {
    image_node(AssetPath::from_path(Path::new("ui")).resolve(&path.into()))
}

#[inline]
pub fn item_image_node(key: &NamespacedKey) -> impl Scene {
    image_node(format!("items/{}", key.path_string()))
}

#[inline]
pub fn fallback<A: Asset>(asset_server: &AssetServer) -> Option<Handle<A>> {
    if TypeId::of::<A>() == TypeId::of::<Image>() {
        Some(asset_server.load("embedded://embers/missingno.png"))
    } else {
        None
    }
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
