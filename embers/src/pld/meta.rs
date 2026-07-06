use super::{
    InjectedPayloads, PayloadManager, Payloads, block_texture, resolve_payload, scan_source_uuid,
};
use crate::dim::MovementsConfig;
use crate::dim::actor::living::attributes::AttributeBase;
use crate::dim::block::{
    BlockCollider, BlockColliderTemplate, BlockModel, BlockVoxelModelTemplate,
};
use crate::dim::item::{
    BoxedItemActionBuilder, BoxedItemComponent, ItemAction, ItemActionBuilder, ItemComponent,
};
use crate::ui::{TextureAnimation, TextureScaling};
use crate::utils::{NamespacedKey, TextureAtlasManifest, path_to_unix_components};
use anyhow::Error;
use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, AssetServer, LoadContext};
use bevy::ecs::system::{StaticSystemInput, StaticSystemParam, SystemParam};
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;
use bevy_tnua::builtins::TnuaBuiltinWalkConfig;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::LazyLock;
use toml::{Table, Value, from_slice};
use uuid::Uuid;

static ACTOR_BASE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"actors/(?P<namespace>[A-Za-z0-9_]+)(?P<key>(?:/[A-Za-z0-9_]+)+)\.actor\.toml$")
        .unwrap()
});

#[derive(Asset, Deserialize, TypePath, Debug)]
struct ActorBase {
    #[serde(default)]
    float_height: f32,
    #[serde(default)]
    attributes: HashMap<NamespacedKey, f32>,
}

static BLOCK_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"blocks/(?P<namespace>[A-Za-z0-9_]+)(?P<key>(?:/[A-Za-z0-9_]+)+)\.block\.toml$")
        .unwrap()
});

#[derive(Asset, Deserialize, TypePath, Debug)]
struct BlockMeta {
    //model: BlockModel,
    collider: BlockColliderMeta,
}

#[derive(Deserialize, Debug)]
struct BlockColliderMeta {
    template: NamespacedKey,
    #[serde(default)]
    config: Table,
}

static ITEM_ACTION_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"item_actions/(?P<namespace>[A-Za-z0-9_]+)(?P<key>(?:/[A-Za-z0-9_]+)+)\.item_action\.toml$")
        .unwrap()
});

#[derive(Asset, Deserialize, TypePath, Debug)]
struct ItemActionMeta {
    template: NamespacedKey,
    config: Table,
}

static ITEM_PROTOTYPE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"items/(?P<namespace>[A-Za-z0-9_]+)(?P<key>(?:/[A-Za-z0-9_]+)+)\.item\.toml$")
        .unwrap()
});

#[derive(Asset, Deserialize, TypePath, Debug)]
struct ItemPrototype(Table);

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

#[derive(TypePath)]
struct TextureAtlasMetadataLoader;

impl AssetLoader for TextureAtlasMetadataLoader {
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
struct RawMetadataLoader<M: Asset + for<'de> Deserialize<'de>>(
    &'static [&'static str],
    PhantomData<M>,
);

impl<M: Asset + for<'de> Deserialize<'de>> RawMetadataLoader<M> {
    pub fn new(extensions: &'static [&'static str]) -> Self {
        Self(extensions, PhantomData)
    }
}

impl<M: Asset + for<'de> Deserialize<'de>> AssetLoader for RawMetadataLoader<M> {
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
pub struct ReloadMetadataRequest;

fn reload_metadata_plugin(app: &mut App) {
    fn process_meta<D: SystemInput + 'static, M: Asset, P: SystemParam + 'static>(
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
                  metadata: Res<Assets<M>>,
                  extra: StaticSystemParam<P>| {
                let mut extra = extra.into_inner();
                for (id, meta) in metadata.iter() {
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
                        meta,
                        &mut extra,
                        &mut data,
                    );
                }
                info!("Found {} {}(s).", metadata.len(), r#type);
                data
            },
        )
    }
    app.add_observer(
        (|_on_reload_metadata: On<ReloadMetadataRequest>,
          mut movements_configs: ResMut<Assets<MovementsConfig>>,
          mut attribute_bases: ResMut<Assets<AttributeBase>>| {
            movements_configs.clear();
            attribute_bases.clear();
            StaticSystemInput(())
        })
        .pipe(process_meta::<
            (),
            ActorBase,
            (
                ResMut<Assets<MovementsConfig>>,
                ResMut<Assets<AttributeBase>>,
            ),
        >(
            &ACTOR_BASE_PATTERN,
            "actor_base",
            |injected_payloads, source_uuid, key, base, (movements, attribute_bases), ()| {
                movements.inject(
                    injected_payloads,
                    source_uuid,
                    format!("movement_configs/{}", key.path_string()),
                    MovementsConfig {
                        basis: TnuaBuiltinWalkConfig {
                            float_height: base.float_height,
                            ..default()
                        },
                        sneak: default(),
                        roll: default(),
                    },
                );
                attribute_bases.inject(
                    injected_payloads,
                    source_uuid,
                    format!("attribute_bases/{}", key.path_string()),
                    AttributeBase::new(base.attributes.clone()),
                );
            },
        )),
    )
    /*.add_observer(
        (|_on_reload_metadata: On<ReloadMetadataRequest>,
          mut block_colliders: RegMut<BlockCollider>,
          mut block_models: RegMut<BlockModel>| {
            block_colliders.clear();
            block_models.clear();
            StaticSystemInput(default())
        })
        .pipe(process_meta::<
            In<TextureAtlasManifest>,
            BlockMeta,
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
    )*/
    .add_observer(
        (|_on_reload_metadata: On<ReloadMetadataRequest>,
          mut item_actions: ResMut<Assets<ItemAction>>| {
            item_actions.clear();
            StaticSystemInput(())
        })
        .pipe(process_meta::<
            (),
            ItemActionMeta,
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
                format!("item_action_builders/{}", action.template.path_string()),
            ) {
                Some(builder) => {
                    item_actions.inject(
                        injected_payloads,
                        source_uuid,
                        format!("item_actions/{}", key.path_string()),
                        builder
                            .build(key, action.config.clone())
                            .expect("Failed to build item action"),
                    );
                }
                None => error!("Unknown item action template: {}", action.template),
            },
        )),
    )
    .add_observer(
        (|_on_reload_metadata: On<ReloadMetadataRequest>| StaticSystemInput(Vec::new()))
            .pipe(process_meta::<In<Vec<_>>, ItemPrototype, ()>(
                &ITEM_PROTOTYPE_PATTERN,
                "item prototype",
                |_injected_payloads,
                 _source_uuid,
                 key,
                 prototype,
                 (),
                 item_prototype_components| {
                    for (component, value) in &prototype.0 {
                        item_prototype_components.push((
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
                |In(item_prototype_components): In<Vec<(_, NamespacedKey, _)>>,
                 mut world: DeferredWorld| {
                    // TODO why the type annotation?
                    for item_component in world
                        .resource::<Assets<BoxedItemComponent>>()
                        .iter()
                        .map(|(_id, item_component)| item_component.dyn_clone())
                        .collect::<Box<[_]>>()
                    {
                        item_component.dyn_clone().clear_prototypes(&mut world);
                    }
                    for (item, item_component, prototype) in item_prototype_components {
                        match resolve_payload(
                            world.resource::<PayloadManager>(),
                            world.resource::<AssetServer>(),
                            world.resource::<Assets<BoxedItemComponent>>(),
                            format!("item_components/{}", item_component.path_string()),
                        ) {
                            Some(item_component) => {
                                item_component
                                    .dyn_clone()
                                    .insert_prototype(&mut world, item, prototype);
                            }
                            None => error!(
                                "Unknown item component key in prototype '{}': {}",
                                item, item_component,
                            ),
                        }
                    }
                },
            ),
    );
}

pub(super) fn plugin(app: &mut App) {
    app.init_asset::<ActorBase>()
        .init_asset::<BlockMeta>()
        .init_asset::<ItemActionMeta>()
        .init_asset::<ItemPrototype>()
        .register_asset_loader(RawMetadataLoader::<ActorBase>::new(&["actor.toml"]))
        .register_asset_loader(RawMetadataLoader::<BlockMeta>::new(&["block.toml"]))
        .register_asset_loader(RawMetadataLoader::<ItemActionMeta>::new(&[
            "item_action.toml",
        ]))
        .register_asset_loader(RawMetadataLoader::<ItemPrototype>::new(&["item.toml"]))
        .register_asset_loader(TextureAtlasMetadataLoader)
        .register_asset_loader(RawMetadataLoader::<TextureAnimation>::new(&[
            "animation.toml",
        ]))
        .register_asset_loader(RawMetadataLoader::<TextureScaling>::new(&["scaling.toml"]))
        .add_plugins(reload_metadata_plugin);
}
