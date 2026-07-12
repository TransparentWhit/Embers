//! *Payload*(pld)*s* are resources that the game uses during execution, such as assets or game data.
//! *Shipment*(shp)*s* are their processed counterpart.

pub mod def;
pub mod foundry;
pub mod manager;

use crate::path;
use crate::utils::{Keyed, NamespacedKey, UniquelyIdentified};
use bevy::app::App;
use bevy::asset::io::AssetSourceId;
use bevy::asset::{AssetPath, embedded_asset};
use bevy::gltf::{GltfMaterial, GltfMesh, GltfNode, GltfSkin};
use bevy::prelude::*;
use bevy_sprinkles::asset::ParticlesAsset;
use delegate::delegate;
use manager::{EMBERS_PAYLOAD_SOURCE_UUID, InjectedPayloads};
use std::collections::HashSet;
use std::ffi::OsStr;
use std::marker::PhantomData;
use uuid::Uuid;

pub trait BoxedPayloadMarker: TypePath {
    fn payload_root() -> AssetPath<'static>;
}

#[derive(Asset, Deref, DerefMut)]
pub struct Boxed<M: BoxedPayloadMarker, T: ?Sized + Send + Sync + 'static> {
    #[deref]
    pub value: Box<T>,
    _marker: PhantomData<fn(M) -> M>,
}

impl<M: BoxedPayloadMarker, T: ?Sized + Send + Sync + 'static> Payload for Boxed<M, T> {
    #[inline]
    fn payload_root() -> AssetPath<'static> {
        M::payload_root()
    }
}

impl<M: BoxedPayloadMarker, T: Send + Sync + 'static> From<T> for Boxed<M, T> {
    #[inline]
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<M: BoxedPayloadMarker, T: ?Sized + Send + Sync + 'static> From<Box<T>> for Boxed<M, T> {
    #[inline]
    fn from(value: Box<T>) -> Self {
        Self::new_boxed(value)
    }
}

impl<M: BoxedPayloadMarker, T: ?Sized + Send + Sync + 'static> TypePath for Boxed<M, T> {
    delegate! {
        to M {
            fn type_path() -> &'static str;
            fn short_type_path() -> &'static str;
            fn type_ident() -> Option<&'static str>;
            fn crate_name() -> Option<&'static str>;
            fn module_path() -> Option<&'static str>;
        }
    }
}

impl<M: BoxedPayloadMarker, T: Clone + ?Sized + Send + Sync + 'static> Clone for Boxed<M, T> {
    fn clone(&self) -> Self {
        Self::new_boxed(self.value.clone())
    }
}

impl<M: BoxedPayloadMarker, T: UniquelyIdentified + ?Sized + Send + Sync + 'static>
    UniquelyIdentified for Boxed<M, T>
{
    fn unique_id(&self) -> Uuid {
        self.value.unique_id()
    }
}

impl<M: BoxedPayloadMarker, T: Keyed + ?Sized + Send + Sync + 'static> Keyed for Boxed<M, T> {
    fn key(&self) -> &NamespacedKey {
        self.value.key()
    }
}

impl<M: BoxedPayloadMarker, T: Send + Sync + 'static> Boxed<M, T> {
    pub fn new(value: T) -> Self {
        Self {
            value: Box::new(value),
            _marker: PhantomData,
        }
    }
}

impl<M: BoxedPayloadMarker, T: ?Sized + Send + Sync + 'static> Boxed<M, T> {
    pub fn new_boxed(value: Box<T>) -> Self {
        Self {
            value,
            _marker: PhantomData,
        }
    }
}

#[derive(Asset, TypePath)]
pub struct Tag<A: Asset> {
    tags: HashSet<AssetPath<'static>>,
    _marker: PhantomData<fn() -> A>,
}

impl<A: Asset> Payload for Tag<A> {
    #[inline]
    fn payload_root() -> AssetPath<'static> {
        "resolved_tags".into()
    }
}

impl<A: Asset> Tag<A> {
    fn new(tags: HashSet<AssetPath<'static>>) -> Self {
        Self {
            tags,
            _marker: PhantomData,
        }
    }
    pub fn contains<'path>(&self, path: impl Into<AssetPath<'path>>) -> bool {
        self.tags.contains(&path.into())
    }
    pub fn iter(&self) -> impl Iterator<Item = &AssetPath<'_>> {
        self.tags.iter()
    }
}

pub trait Payload: Asset {
    fn payload_root() -> AssetPath<'static>;
}

impl Payload for AnimationClip {
    fn payload_root() -> AssetPath<'static> {
        "models".into()
    }
}

impl Payload for AudioSource {
    fn payload_root() -> AssetPath<'static> {
        "sounds".into()
    }
}

impl Payload for Font {
    fn payload_root() -> AssetPath<'static> {
        "fonts".into()
    }
}

impl Payload for Gltf {
    fn payload_root() -> AssetPath<'static> {
        "models".into()
    }
}

impl Payload for GltfMaterial {
    fn payload_root() -> AssetPath<'static> {
        "models".into()
    }
}

impl Payload for GltfMesh {
    fn payload_root() -> AssetPath<'static> {
        "models".into()
    }
}

impl Payload for GltfNode {
    fn payload_root() -> AssetPath<'static> {
        "models".into()
    }
}

impl Payload for GltfSkin {
    fn payload_root() -> AssetPath<'static> {
        "models".into()
    }
}

impl Payload for Image {
    fn payload_root() -> AssetPath<'static> {
        "textures".into()
    }
}

impl Payload for ParticlesAsset {
    fn payload_root() -> AssetPath<'static> {
        "particles".into()
    }
}

impl Payload for Shader {
    fn payload_root() -> AssetPath<'static> {
        "shaders".into()
    }
}

impl Payload for TextureAtlasLayout {
    fn payload_root() -> AssetPath<'static> {
        "textures".into()
    }
}

impl Payload for WorldAsset {
    fn payload_root() -> AssetPath<'static> {
        "models".into()
    }
}

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

pub trait Payloads<P: Payload> {
    fn inject<'path>(
        &mut self,
        injected_payloads: &mut InjectedPayloads,
        source_uuid: Uuid,
        path: impl Into<AssetPath<'path>>,
        payload: P,
    );
    fn clear(&mut self);
}

impl<P: Payload> Payloads<P> for Assets<P> {
    fn inject<'path>(
        &mut self,
        injected_payloads: &mut InjectedPayloads,
        source_uuid: Uuid,
        path: impl Into<AssetPath<'path>>,
        payload: P,
    ) {
        let uuid = Uuid::new_v5(
            &source_uuid,
            P::payload_root()
                .resolve(&path.into())
                .with_source(AssetSourceId::Default)
                .to_string()
                .as_bytes(),
        );
        injected_payloads.source_uuids.insert(uuid, source_uuid);
        self.insert(uuid, payload).unwrap();
    }
    fn clear(&mut self) {
        for id in self.ids().collect::<Box<[_]>>() {
            self.remove(id);
        }
    }
}

pub(crate) trait EmbersPayloads<P: Payload>: Payloads<P> {
    fn inject_embers<'path>(
        &mut self,
        injected_payloads: &mut InjectedPayloads,
        path: impl Into<AssetPath<'path>>,
        payload: P,
    );
}

impl<P: Payload> EmbersPayloads<P> for Assets<P> {
    #[inline]
    fn inject_embers<'path>(
        &mut self,
        injected_payloads: &mut InjectedPayloads,
        path: impl Into<AssetPath<'path>>,
        payload: P,
    ) {
        self.inject(injected_payloads, EMBERS_PAYLOAD_SOURCE_UUID, path, payload);
    }
}

pub trait PayloadApp {
    fn init_tags<A: Asset>(&mut self) -> &mut Self;
}

impl<App: AssetApp> PayloadApp for App {
    #[inline]
    fn init_tags<A: Asset>(&mut self) -> &mut Self {
        self.init_asset::<Tag<A>>()
    }
}

pub(super) fn plugin(app: &mut App) {
    embedded_asset!(app, path!("icon.png"));
    embedded_asset!(app, path!("missingno.png"));
    app.add_plugins((def::plugin, manager::plugin));
}
