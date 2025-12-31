use crate::dim::actor::living::AttributeBase;
use crate::dim::item::{ItemAction, ItemActionTemplate, ItemComponent};
use crate::reg::{DynReg, DynamicRegistry, RegMut, Registry};
use crate::utils::{NamespacedKey, path_to_unix_components};
use anyhow::Error;
use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, AssetServer, LoadContext};
use bevy::ecs::system::{SystemId, SystemState};
use bevy::prelude::*;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::LazyLock;
use toml::{Table, from_slice};

#[derive(Resource)]
pub struct ReloadMetadata(SystemId);

impl ReloadMetadata {
    pub fn system_id(&self) -> SystemId {
        self.0
    }
}

static ACTOR_BASE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"actors/(?P<namespace>[A-Za-z0-9_]+)(?P<key>(?:/[A-Za-z0-9_]+)+)\.actor\.toml$")
        .unwrap()
});

#[derive(Asset, Debug, Deserialize, TypePath)]
struct ActorBase {
    #[serde(default)]
    attributes: Option<HashMap<NamespacedKey, f32>>,
}

static ITEM_ACTION_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"item_actions/(?P<namespace>[A-Za-z0-9_]+)(?P<key>(?:/[A-Za-z0-9_]+)+)\.item_action\.toml$")
        .unwrap()
});

#[derive(Asset, Debug, Deserialize, TypePath)]
struct ItemActionMeta {
    template: NamespacedKey,
    config: Table,
}

static ITEM_PROTOTYPE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"items/(?P<namespace>[A-Za-z0-9_]+)(?P<key>(?:/[A-Za-z0-9_]+)+)\.item\.toml$")
        .unwrap()
});

#[derive(Asset, Debug, Deserialize, TypePath)]
struct ItemPrototype(Table);

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

pub fn reload_metadata(
    world: &mut World,
    state: &mut SystemState<(
        Res<AssetServer>,
        (Res<Assets<ActorBase>>, RegMut<AttributeBase>),
        (
            Res<Assets<ItemActionMeta>>,
            DynReg<dyn ItemActionTemplate>,
            RegMut<ItemAction>,
        ),
        Res<Assets<ItemPrototype>>,
    )>,
) {
    let (
        asset_server,
        (actor_bases, mut attribute_bases),
        (item_action_metas, item_action_templates, mut item_actions),
        item_prototypes,
    ) = state.get_mut(world);
    fn process_meta<M: Asset>(
        asset_server: &AssetServer,
        metadata: &Assets<M>,
        path_pattern: &Regex,
        r#type: &str,
        parse: &mut impl FnMut(NamespacedKey, &M),
    ) {
        for (id, meta) in metadata.iter() {
            let path = path_to_unix_components(asset_server.get_path(id).unwrap().path());
            parse(
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
            );
        }
        info!("Found {} {}(s).", metadata.len(), r#type);
    }
    *attribute_bases = Registry::new();
    process_meta(
        &asset_server,
        &actor_bases,
        &ACTOR_BASE_PATTERN,
        "actor base",
        &mut |key, base| {
            attribute_bases
                .register(
                    key,
                    AttributeBase::new(base.attributes.as_ref().cloned().unwrap_or_default()),
                )
                .expect("Failed to register attribute bases");
        },
    );
    *item_actions = Registry::new();
    process_meta(
        &asset_server,
        &item_action_metas,
        &ITEM_ACTION_PATTERN,
        "item action",
        &mut |key, action| match item_action_templates.get(&action.template) {
            Some(template) => item_actions
                .register_keyed(
                    template
                        .create(key, action.config.clone())
                        .expect("Failed to apply item action template"),
                )
                .expect("Failed to register item action"),
            None => error!("Unknown item action template: {}", action.template),
        },
    );
    let mut item_prototype_components = Vec::new();
    process_meta(
        &asset_server,
        &item_prototypes,
        &ITEM_PROTOTYPE_PATTERN,
        "item prototype",
        &mut |key, prototype| {
            for (component, value) in &prototype.0 {
                let component = match NamespacedKey::try_from_with_embers(component.as_str()) {
                    Ok(key) => key,
                    Err(err) => {
                        error!("Invalid item component key in {}: {}", key, err);
                        continue;
                    }
                };
                item_prototype_components.push((key.clone(), component.clone(), value.clone()));
            }
        },
    );
    world.resource_scope::<DynamicRegistry<dyn ItemComponent>, ()>(|world, item_components| {
        for item_component in item_components.iter() {
            item_component.reset_registry(world);
        }
        for (item, item_component, value) in item_prototype_components {
            match item_components.get(&item_component) {
                Some(item_component) => item_component.register_prototype(world, item, value),
                None => error!(
                    "Unknown item component key in prototype '{}': {}",
                    item, item_component,
                ),
            }
        }
    });
    info!("Finished loading metadata.");
}

pub(super) fn plugin(app: &mut App) {
    let reload_metadata = ReloadMetadata(app.register_system(reload_metadata));
    app.init_asset::<ActorBase>()
        .register_asset_loader(MetadataLoader::<ActorBase>::new(&["actor.toml"]))
        .init_asset::<ItemActionMeta>()
        .register_asset_loader(MetadataLoader::<ItemActionMeta>::new(&["item_action.toml"]))
        .init_asset::<ItemPrototype>()
        .register_asset_loader(MetadataLoader::<ItemPrototype>::new(&["item.toml"]))
        .insert_resource(reload_metadata);
}
