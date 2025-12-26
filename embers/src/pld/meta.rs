use crate::dim::actor::living::AttributeBase;
use crate::dim::item::ItemComponent;
use crate::pld::MetadataLoader;
use crate::reg::{DynamicRegistry, Registry};
use crate::utils::{NamespacedKey, path_to_unix_components};
use bevy::asset::AssetServer;
use bevy::ecs::system::{SystemId, SystemState};
use bevy::prelude::*;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::LazyLock;
use toml::Table;

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
    attributes: Option<HashMap<NamespacedKey, f32>>,
}

static ITEM_PROTOTYPE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"items/(?P<namespace>[A-Za-z0-9_]+)(?P<key>(?:/[A-Za-z0-9_]+)+)\.item\.toml$")
        .unwrap()
});

#[derive(Asset, Debug, Deserialize, TypePath)]
struct ItemPrototype(Table);

pub fn reload_metadata(
    world: &mut World,
    state: &mut SystemState<(
        Res<AssetServer>,
        (Res<Assets<ActorBase>>, ResMut<Registry<AttributeBase>>),
        Res<Assets<ItemPrototype>>,
    )>,
) {
    let (asset_server, (actor_bases, mut attribute_bases), item_prototypes) = state.get_mut(world);
    *attribute_bases = Registry::new();
    for (id, base) in actor_bases.iter() {
        let path = path_to_unix_components(asset_server.get_path(id).unwrap().path());
        let actor = match ACTOR_BASE_PATTERN.captures(&path) {
            Some(captures) => NamespacedKey::new(
                captures.name("namespace").unwrap().as_str(),
                &captures.name("key").unwrap().as_str()[1..],
            ),
            None => {
                error!("Failed to resolve actor key for path {}", path);
                continue;
            }
        };
        attribute_bases
            .register(
                actor.clone(),
                AttributeBase::new(base.attributes.as_ref().cloned().unwrap_or_default()),
            )
            .expect(&format!(
                "Failed to register attribute bases for actor '{}'",
                actor
            ));
    }
    info!("Found {} actor base(s).", actor_bases.len());
    let mut item_prototype_components = Vec::new();
    for (id, prototype) in item_prototypes.iter() {
        let path = path_to_unix_components(asset_server.get_path(id).unwrap().path());
        let item = match ITEM_PROTOTYPE_PATTERN.captures(&path) {
            Some(captures) => NamespacedKey::new(
                captures.name("namespace").unwrap().as_str(),
                &captures.name("key").unwrap().as_str()[1..],
            ),
            None => {
                error!("Failed to resolve item key for path {}", path);
                continue;
            }
        };
        for (component, value) in &prototype.0 {
            let component = match NamespacedKey::try_from_with_embers(component.as_str()) {
                Ok(key) => key,
                Err(err) => {
                    error!("Invalid item component key in {}: {}", item, err);
                    continue;
                }
            };
            item_prototype_components.push((item.clone(), component.clone(), value.clone()));
        }
    }
    info!("Found {} item prototype(s).", item_prototypes.len());
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
        .init_asset::<ItemPrototype>()
        .register_asset_loader(MetadataLoader::<ItemPrototype>::new(&["item.toml"]))
        .insert_resource(reload_metadata);
}
