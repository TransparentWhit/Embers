use super::manager::{PayloadManager, resolve_handle};
use super::{Payload, PayloadPath};
use crate::ui::{AnimatedTexture, TextureAnimation, TextureScaling};
use crate::utils::{Keyed, NamespacedKey, remove_bundle};
use atomicow::CowArc;
use bevy::asset::AssetPath;
use bevy::ecs::template::{TemplateContext, TemplateTuple};
use bevy::gltf::{GltfMaterial, GltfMesh, GltfNode, GltfSkin};
use bevy::prelude::*;
use derive_where::derive_where;
use std::any::TypeId;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[inline]
pub fn pld<P: Payload>(value: impl Into<PayloadTemplate<P>>) -> PayloadTemplate<P> {
    value.into()
}

#[inline]
pub fn optional_pld<P: Payload>(
    value: impl Into<PayloadTemplate<P>>,
) -> OptionalPayloadTemplate<P> {
    OptionalPayloadTemplate(pld(value))
}

#[inline]
pub fn payload_value<P: Payload>(value: impl Into<P>) -> PayloadTemplate<P> {
    PayloadTemplate::value(value.into())
}

#[derive(Debug)]
pub enum PayloadTemplate<P: Payload> {
    Handle(Handle<P>),
    Path(AssetPath<'static>),
    Value(Arc<Mutex<Result<Option<P>, Handle<P>>>>),
}

impl<P: Payload> Default for PayloadTemplate<P> {
    fn default() -> Self {
        Self::Handle(default())
    }
}

impl<P: Payload> From<Handle<P>> for PayloadTemplate<P> {
    fn from(value: Handle<P>) -> Self {
        Self::Handle(value)
    }
}

impl<P: Payload> From<AssetPath<'_>> for PayloadTemplate<P> {
    fn from(value: AssetPath<'_>) -> Self {
        Self::Path(value.into_owned())
    }
}

impl<P: Payload> PayloadTemplate<P> {
    pub fn handle(value: impl Into<Handle<P>>) -> Self {
        Self::Handle(value.into())
    }
    pub fn path<'path>(value: impl Into<AssetPath<'path>>) -> Self {
        Self::Path(value.into().into_owned())
    }
    pub fn value(value: impl Into<P>) -> Self {
        Self::Value(Arc::new(Mutex::new(Ok(Some(value.into())))))
    }
}

#[derive(Debug, Error)]
#[error("Couldn't find payload of type `{type_path}` in {path}")]
struct PayloadNotFoundError {
    path: AssetPath<'static>,
    type_path: String,
}

impl<P: Payload> Template for PayloadTemplate<P> {
    type Output = Handle<P>;
    fn build_template(&self, context: &mut TemplateContext) -> Result<Self::Output> {
        match self {
            Self::Handle(handle) => Ok(handle.clone()),
            Self::Path(path) => resolve_handle(
                context.entity.resource::<PayloadManager>(),
                context.entity.resource::<AssetServer>(),
                context.entity.resource::<Assets<P>>(),
                path,
            )
            .or_else(|| fallback(context.entity.resource::<AssetServer>()))
            .ok_or_else(|| {
                BevyError::error(PayloadNotFoundError {
                    path: path.clone(),
                    type_path: P::short_type_path().to_string(),
                })
            }),
            Self::Value(value_or_handle) => {
                let value_or_handle = &mut *value_or_handle.lock().unwrap();
                match value_or_handle {
                    Ok(value) => {
                        let handle = context
                            .resource_mut::<Assets<P>>()
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
pub struct OptionalPayloadTemplate<P: Payload>(pub PayloadTemplate<P>);

impl<P: Payload> Template for OptionalPayloadTemplate<P> {
    type Output = Option<Handle<P>>;
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
        font: pld(AssetPath::from(key.key()).with_added_extension("ttf")),
        size: size.into(),
    })
}

#[inline]
fn model<'path>(path: impl Into<AssetPath<'path>>) -> PayloadTemplate<Gltf> {
    pld(path.into().with_added_extension("glb"))
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
    _marker: PhantomData<fn() -> E>,
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
    animate(AssetPath::from("actors").resolve(&key.into()), id)
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
pub fn default_scene() -> impl Scene {
    // TODO use embedded bsn when bsn reader comes out
    bsn! {
        template_value(WorldAssetRootTemplate {
            scene: model_element("missingno", GltfElementId::Default),
        })
    }
}

#[inline]
pub fn actor_scene(key: &NamespacedKey, id: GltfElementId) -> impl Scene {
    bsn! {
        template_value(WorldAssetRootTemplate {
            scene: model_element(AssetPath::from("actors").resolve(&key.into()), id),
        })
    }
}

#[inline]
fn plain_image<'path>(path: impl Into<AssetPath<'path>>) -> PayloadTemplate<Image> {
    pld(path.into().with_added_extension("png"))
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
    let path = path.into();
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
    rich_image(AssetPath::from("blocks").resolve(&key.into()))
}

#[derive(Default)]
struct ImageNodeTemplate {
    image: PayloadTemplate<Image>,
    atlas: OptionalPayloadTemplate<TextureAtlasLayout>,
    animation: OptionalPayloadTemplate<TextureAnimation>,
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
        if let Some(animation) = self.animation.build_template(context).ok().flatten() {
            context.entity.insert(AnimatedTexture::new(
                context
                    .resource::<Assets<TextureAnimation>>()
                    .get(&animation)
                    .unwrap()
                    .clone(),
            ));
        } else {
            context.entity.remove::<AnimatedTexture>();
        }
        Ok(node)
    }
    fn clone_template(&self) -> Self {
        Self {
            image: self.image.clone_template(),
            atlas: self.atlas.clone_template(),
            animation: self.animation.clone_template(),
            scaling: self.scaling.clone_template(),
        }
    }
}

#[inline]
fn image_node<'path>(path: impl Into<AssetPath<'path>>) -> impl Scene {
    let TemplateTuple((image, atlas, animation, scaling)) = rich_image(path);
    bsn! {
        template_value(ImageNodeTemplate { image, atlas, animation, scaling })
    }
}

#[inline]
pub fn empty_image_node() -> impl Scene {
    remove_bundle::<(ImageNode, AnimatedTexture)>()
}

#[inline]
pub fn ui_image_node<'path>(path: impl Into<AssetPath<'path>>) -> impl Scene {
    image_node(AssetPath::from("ui").resolve(&path.into()))
}

#[inline]
pub fn item_image_node(key: &NamespacedKey) -> impl Scene {
    image_node(AssetPath::from("items").resolve(&key.into()))
}

#[inline]
fn fallback<A: Asset>(asset_server: &AssetServer) -> Option<Handle<A>> {
    if TypeId::of::<A>() == TypeId::of::<Image>() {
        Some(asset_server.load("embedded://embers/missingno.png"))
    } else {
        None
    }
}
