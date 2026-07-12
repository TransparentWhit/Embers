use super::foundry::block_texture;
use super::manager::{InjectedPayloads, PayloadManager, resolve_payload, scan_source_uuid};
use super::{Payload, Payloads, Tag};
use crate::dim::MovementsConfig;
use crate::dim::actor::living::attributes::AttributeBase;
use crate::dim::block::{
    BlockCollider, BlockColliderTemplate, BlockModel, BlockVoxelModelTemplate,
};
use crate::dim::item::{BoxedItemActionBuilder, BoxedItemComponentType, ItemAction};
use crate::ui::{TextureAnimation, TextureScaling};
use crate::utils::{NamespacedKey, TextureAtlasManifest, path_to_unix_components};
use anyhow::Error;
use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, AssetPath, AssetServer, LoadContext};
use bevy::ecs::system::{
    IntoObserverSystem, ObserverSystem, StaticSystemInput, StaticSystemParam, SystemParam,
};
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;
use bevy_tnua::builtins::TnuaBuiltinWalkConfig;
use regex::Regex;
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::LazyLock;
use toml::{Table, Value, from_slice};
use uuid::Uuid;

fn compile_definitions<D: SystemInput + 'static, M: Asset, P: SystemParam + 'static>(
    path_pattern: &'static Regex,
    r#type: &'static str,
    parse: fn(
        &mut InjectedPayloads,
        Uuid,
        NamespacedKey,
        &M,
        &mut P::Item<'_, '_>,
        &mut D::Inner<'_>,
    ),
) -> impl System<In = In<StaticSystemInput<'static, D>>, Out = D::Inner<'static>> {
    IntoSystem::into_system(
        move |In(StaticSystemInput(mut data)): In<StaticSystemInput<'static, D>>,
              mut injected_payloads: ResMut<InjectedPayloads>,
              payload_manager: Res<PayloadManager>,
              asset_server: Res<AssetServer>,
              definitions: Res<Assets<M>>,
              extra: StaticSystemParam<P>| {
            let mut extra = extra.into_inner();
            for (id, definition) in definitions.iter() {
                let Some(source_uuid) =
                    scan_source_uuid(&injected_payloads, &payload_manager, &asset_server, id)
                else {
                    continue;
                };
                let path = path_to_unix_components(asset_server.get_path(id).unwrap().path());
                parse(
                    &mut injected_payloads,
                    source_uuid,
                    match path_pattern.captures(&path) {
                        Some(captures) => NamespacedKey::new(
                            captures.name("namespace").unwrap().as_str(),
                            &captures.name("key").unwrap().as_str()[1..],
                        ),
                        None => {
                            error!("Failed to resolve {} key for path {}", r#type, path);
                            continue;
                        }
                    },
                    definition,
                    &mut extra,
                    &mut data,
                );
            }
            info!("Found {} {}(s).", definitions.len(), r#type);
            data
        },
    )
}

static ACTOR_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"actors/(?P<namespace>[A-Za-z0-9_]+)(?P<key>(?:/[A-Za-z0-9_]+)+)\.actor\.toml$")
        .unwrap()
});

#[derive(Asset, Deserialize, TypePath, Debug)]
pub(super) struct ActorDef {
    #[serde(default)]
    float_height: f32,
    #[serde(default)]
    attributes: HashMap<NamespacedKey, f32>,
}

impl Payload for ActorDef {
    fn payload_root() -> AssetPath<'static> {
        "actors".into()
    }
}

fn recompile_actors() -> impl ObserverSystem<RecompileDefinitionsRequest, ()> {
    IntoObserverSystem::into_system(
        (|_request: On<RecompileDefinitionsRequest>,
          mut movements_configs: ResMut<Assets<MovementsConfig>>,
          mut attribute_bases: ResMut<Assets<AttributeBase>>| {
            movements_configs.clear();
            attribute_bases.clear();
            StaticSystemInput(())
        })
        .pipe(compile_definitions::<
            (),
            ActorDef,
            (
                ResMut<Assets<MovementsConfig>>,
                ResMut<Assets<AttributeBase>>,
            ),
        >(
            &ACTOR_PATTERN,
            "actor",
            |injected_payloads, source_uuid, key, base, (movements, attribute_bases), ()| {
                movements.inject(
                    injected_payloads,
                    source_uuid,
                    &key,
                    MovementsConfig {
                        basis: TnuaBuiltinWalkConfig {
                            float_height: base.float_height,
                            ..default()
                        },
                        knockback: default(),
                        sneak: default(),
                        roll: default(),
                    },
                );
                attribute_bases.inject(
                    injected_payloads,
                    source_uuid,
                    &key,
                    AttributeBase::new(base.attributes.clone()),
                );
            },
        )),
    )
}

static BLOCK_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"blocks/(?P<namespace>[A-Za-z0-9_]+)(?P<key>(?:/[A-Za-z0-9_]+)+)\.block\.toml$")
        .unwrap()
});

#[derive(Asset, Deserialize, TypePath, Debug)]
pub(super) struct BlockDef {
    //model: BlockModel,
    collider: BlockColliderDef,
}

impl Payload for BlockDef {
    fn payload_root() -> AssetPath<'static> {
        "blocks".into()
    }
}

#[derive(Deserialize, Debug)]
struct BlockColliderDef {
    template: NamespacedKey,
    #[serde(default)]
    config: Table,
}

/*fn recompile_blocks() -> impl ObserverSystem<RecompileDefinitionsRequest, ()> {
    IntoObserverSystem::into_system((|_request: On<RecompileDefinitionsRequest>,
          mut block_colliders: RegMut<BlockCollider>,
          mut block_models: RegMut<BlockModel>| {
            block_colliders.clear();
            block_models.clear();
            StaticSystemInput(default())
        })
        .pipe(compile_definitions::<
            In<TextureAtlasManifest>,
            BlockDef,
            (
                Res<PayloadManager>,
                Res<AssetServer>,
                Res<Assets<Image>>,
                Res<Assets<TextureAtlasLayout>>,
                Res<Assets<TextureAnimation>>,
                Res<Assets<TextureScaling>>,
                RegMut<BlockCollider>,
                RegBoxed<dyn BlockColliderTemplate>,
                RegMut<BlockModel>,
                RegBoxed<dyn BlockVoxelModelTemplate>,
            ),
        >(
            &BLOCK_PATTERN,
            "block",
            |injected_payloads, source_uuid, key,
             block,
             (
                payload_manager,
                asset_server,
                images,
                texture_atlas_alyouts,
                texture_animations,
                texture_scalings,
                block_colliders,
                block_collider_templates,
                block_models,
                block_voxel_model_templates,
            ),
             block_atlas_manifest| {
                match block_collider_templates.get(&block.collider.template) {
                    Some(block_collider_template) => {
                        block_colliders
                            .register(
                                key.clone(),
                                block_collider_template
                                    .create(block.collider.config.clone())
                                    .expect("Failed to apply block collider template"),
                            )
                            .expect("Failed to register block collider");
                    }
                    None => error!(
                        "Unknown block collider template: {}",
                        block.collider.template
                    ),
                }
                /*let (image, atlas, animation, scaling) = block_texture(&payload_manager, &asset_server, &key);
                // TODO animations
                block_atlas_manifest.add_texture(Some(image.id()), image);*/
            },
        ))
        .pipe(
            |In(block_atlas_manifest): In<TextureAtlasManifest>, images: Res<Assets<Image>>| {
                block_atlas_manifest
                    .manifest(&images)
                    .unwrap()
                    .build()
                    .unwrap();
            },
        ),
    )
}*/

static ITEM_ACTION_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"item_actions/(?P<namespace>[A-Za-z0-9_]+)(?P<key>(?:/[A-Za-z0-9_]+)+)\.item_action\.toml$")
        .unwrap()
});

#[derive(Asset, Deserialize, TypePath, Debug)]
pub(super) struct ItemActionDef {
    template: NamespacedKey,
    config: Table,
}

impl Payload for ItemActionDef {
    fn payload_root() -> AssetPath<'static> {
        "item_actions".into()
    }
}

fn recompile_item_actions() -> impl ObserverSystem<RecompileDefinitionsRequest, ()> {
    IntoObserverSystem::into_system(
        (|_request: On<RecompileDefinitionsRequest>,
          mut item_actions: ResMut<Assets<ItemAction>>| {
            item_actions.clear();
            StaticSystemInput(())
        })
        .pipe(compile_definitions::<
            (),
            ItemActionDef,
            (
                Res<PayloadManager>,
                Res<AssetServer>,
                Res<Assets<BoxedItemActionBuilder>>,
                ResMut<Assets<ItemAction>>,
            ),
        >(
            &ITEM_ACTION_PATTERN,
            "item action",
            |injected_payloads,
             source_uuid,
             key,
             action,
             (payload_manager, asset_server, item_action_builders, item_actions),
             ()| match resolve_payload(
                &payload_manager,
                &asset_server,
                item_action_builders,
                &action.template,
            ) {
                Some(builder) => {
                    item_actions.inject(
                        injected_payloads,
                        source_uuid,
                        &key.clone(),
                        builder
                            .build(key, action.config.clone())
                            .expect("Failed to build item action"),
                    );
                }
                None => error!("Unknown item action template: {}", action.template),
            },
        )),
    )
}

static ITEM_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"items/(?P<namespace>[A-Za-z0-9_]+)(?P<key>(?:/[A-Za-z0-9_]+)+)\.item\.toml$")
        .unwrap()
});

#[derive(Asset, Deserialize, TypePath, Debug)]
pub(super) struct ItemDef(Table);

impl Payload for ItemDef {
    fn payload_root() -> AssetPath<'static> {
        "items".into()
    }
}

fn recompile_items() -> impl ObserverSystem<RecompileDefinitionsRequest, ()> {
    IntoObserverSystem::into_system(
        (|_request: On<RecompileDefinitionsRequest>| StaticSystemInput(Vec::new()))
            .pipe(compile_definitions::<In<Vec<_>>, ItemDef, ()>(
                &ITEM_PATTERN,
                "item",
                |_injected_payloads, _source_uuid, key, item, (), item_component_prototypes| {
                    for (component, value) in &item.0 {
                        item_component_prototypes.push((
                            key.clone(),
                            match NamespacedKey::try_from_with_embers(component.as_str()) {
                                Ok(key) => key,
                                Err(err) => {
                                    error!("Invalid item component key in {}: {}", key, err);
                                    continue;
                                }
                            },
                            value.clone(),
                        ));
                    }
                },
            ))
            .pipe(
                |In(item_component_prototypes): In<Vec<(_, NamespacedKey, _)>>,
                 mut world: DeferredWorld| {
                    // TODO why the type annotation?
                    for item_component in world
                        .resource::<Assets<BoxedItemComponentType>>()
                        .iter()
                        .map(|(_id, item_component)| item_component.dyn_clone())
                        .collect::<Box<[_]>>()
                    {
                        item_component.dyn_clone().clear_prototypes(&mut world);
                    }
                    for (item, component, prototype) in item_component_prototypes {
                        match resolve_payload(
                            world.resource::<PayloadManager>(),
                            world.resource::<AssetServer>(),
                            world.resource::<Assets<BoxedItemComponentType>>(),
                            &component,
                        ) {
                            Some(item_component) => {
                                item_component
                                    .dyn_clone()
                                    .inject_prototype(&mut world, &item, prototype);
                            }
                            None => error!(
                                "Unknown item component key in item '{}': {}",
                                item, component,
                            ),
                        }
                    }
                },
            ),
    )
}

static PARTICLE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"particles/(?P<namespace>[A-Za-z0-9_]+)(?P<key>(?:/[A-Za-z0-9_]+)+)\.particle\.toml$",
    )
    .unwrap()
});

static VOXEL_MODEL_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"models/voxels/(?P<namespace>[A-Za-z0-9_]+)(?P<key>(?:/[A-Za-z0-9_]+)+)\.voxel\.toml$",
    )
    .unwrap()
});

#[derive(Asset, Deserialize, TypePath, Debug)]
struct VoxelModel {
    parent: Option<NamespacedKey>,
    #[serde(default)]
    images: HashMap<String, NamespacedKey>,
}

#[derive(Asset, Deserialize, TypePath, Debug)]
struct ParticleMeta {
    /*max_particles: u32,
    spawn_count: f32,
    spawn_duration_secs: f32,
    spawn_period_secs: f32,
    spawn_cycles: u32,*/
}

static TAG_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"tags/(?P<namespace>[A-Za-z0-9_]+)(?P<key>(?:/[A-Za-z0-9_]+)+)\.tag\.toml$")
        .unwrap()
});

enum TagEntry {
    Tag(AssetPath<'static>),
    Value(AssetPath<'static>),
}

impl<'de> Deserialize<'de> for TagEntry {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let string = String::deserialize(deserializer)?;
        if let Some(stripped) = string.strip_prefix('#') {
            Ok(TagEntry::Tag(AssetPath::from(stripped.to_string())))
        } else {
            Ok(TagEntry::Value(AssetPath::from(string)))
        }
    }
}

#[derive(Asset, Deserialize, TypePath)]
pub(super) struct TagDef {
    entries: Vec<TagEntry>,
}

impl Payload for TagDef {
    fn payload_root() -> AssetPath<'static> {
        "tags".into()
    }
}

// TODO Tag resolution

#[derive(TypePath)]
struct TextureAtlasDefLoader;

impl AssetLoader for TextureAtlasDefLoader {
    type Asset = TextureAtlasLayout;
    type Settings = ();
    type Error = Error;
    async fn load(
        &self,
        reader: &mut dyn Reader,
        (): &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        #[non_exhaustive]
        #[derive(Deserialize)]
        #[serde(rename_all = "snake_case", tag = "type")]
        enum TextureAtlasPreset {
            Grid {
                tile: UVec2,
                columns: u32,
                rows: u32,
                padding: Option<UVec2>,
                offset: Option<UVec2>,
            },
        }
        impl From<TextureAtlasPreset> for TextureAtlasLayout {
            fn from(value: TextureAtlasPreset) -> Self {
                match value {
                    TextureAtlasPreset::Grid {
                        tile,
                        columns,
                        rows,
                        padding,
                        offset,
                    } => Self::from_grid(tile, columns, rows, padding, offset),
                }
            }
        }
        Ok(from_slice::<TextureAtlasPreset>(&bytes)?.into())
    }
    fn extensions(&self) -> &[&str] {
        &["atlas.toml"]
    }
}

#[derive(TypePath)]
struct RawAssetLoader<M: Asset + for<'de> Deserialize<'de>>(
    &'static [&'static str],
    PhantomData<M>,
);

impl<M: Asset + for<'de> Deserialize<'de>> RawAssetLoader<M> {
    pub fn new(extensions: &'static [&'static str]) -> Self {
        Self(extensions, PhantomData)
    }
}

impl<M: Asset + for<'de> Deserialize<'de>> AssetLoader for RawAssetLoader<M> {
    type Asset = M;
    type Settings = ();
    type Error = Error;
    async fn load(
        &self,
        reader: &mut dyn Reader,
        (): &Self::Settings,
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

#[derive(Event)]
pub struct RecompileDefinitionsRequest;

pub(super) fn plugin(app: &mut App) {
    app.init_asset::<ActorDef>()
        .register_asset_loader(RawAssetLoader::<ActorDef>::new(&["actor.toml"]))
        .add_observer(recompile_actors())
        .init_asset::<BlockDef>()
        .register_asset_loader(RawAssetLoader::<BlockDef>::new(&["block.toml"]))
        .init_asset::<ItemActionDef>()
        .register_asset_loader(RawAssetLoader::<ItemActionDef>::new(&["item_action.toml"]))
        .add_observer(recompile_item_actions())
        .init_asset::<ItemDef>()
        .register_asset_loader(RawAssetLoader::<ItemDef>::new(&["item.toml"]))
        .add_observer(recompile_items())
        .init_asset::<TagDef>()
        .register_asset_loader(RawAssetLoader::<TagDef>::new(&["tag.toml"]))
        .register_asset_loader(TextureAtlasDefLoader)
        .register_asset_loader(RawAssetLoader::<TextureAnimation>::new(&["animation.toml"]))
        .register_asset_loader(RawAssetLoader::<TextureScaling>::new(&["scaling.toml"]));
}
