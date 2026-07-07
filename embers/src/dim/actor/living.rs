pub mod attributes;
pub mod creeper;
pub mod dummy;
pub mod player;

use super::actor;
use crate::dim::{Movements, MovementsConfig, PhysicsPreset};
use crate::pld::PayloadTemplate;
use crate::utils::NamespacedKey;
use attributes::{Attributes, AttributesTemplate, MaxHealth};
use bevy::ecs::template::TemplateContext;
use bevy::prelude::*;
use bevy::scene::SceneFunction;
use bevy_tnua::prelude::*;

#[derive(Component, Debug, Default)]
pub struct LivingActor;

#[derive(Default)]
struct MovementConfigTemplate {
    config: PayloadTemplate<MovementsConfig>,
}

impl Template for MovementConfigTemplate {
    type Output = TnuaConfig<Movements>;
    fn build_template(&self, context: &mut TemplateContext) -> Result<Self::Output> {
        Ok(TnuaConfig(self.config.build_template(context)?))
    }
    fn clone_template(&self) -> Self {
        Self {
            config: self.config.clone_template(),
        }
    }
}

impl MovementConfigTemplate {
    fn new(actor_key: &NamespacedKey) -> Self {
        Self {
            config: PayloadTemplate::path(format!("movement_configs/{}", actor_key.path_string())),
        }
    }
}

#[derive(Component, Debug)]
#[require(LivingActor)]
pub struct Health(pub f32);

impl FromTemplate for Health {
    type Template = HealthTemplate;
}

#[derive(Default)]
/// # Note
/// HealthTemplate depends on [`AttributesTemplate`], which is a bundle template, so HealthTemplate
/// has to be used as a bundle template as well, as bundle templates are applied after component
/// templates.
pub struct HealthTemplate;

impl Template for HealthTemplate {
    type Output = Health;
    fn build_template(&self, context: &mut TemplateContext) -> Result<Self::Output> {
        Ok(Health(
            context
                .entity
                .get::<Attributes<MaxHealth>>()
                .unwrap()
                .value(),
        ))
    }
    fn clone_template(&self) -> Self {
        Self
    }
}

pub fn living_actor(key: &NamespacedKey, interactable: bool) -> impl Scene {
    let key_cloned = key.clone();
    let attributes = SceneFunction(move |_scene_context, resolved| {
        resolved.push_bundle_template(AttributesTemplate::new(key_cloned.clone()));
    });
    let health = SceneFunction(move |_scene_context, resolved| {
        resolved.push_bundle_template(HealthTemplate);
    });
    bsn! {
        actor()
        { PhysicsPreset::LivingActor.physics(interactable) }
        attributes
        health
        template(|_| Ok(TnuaController::<Movements>::default()))
        template_value(MovementConfigTemplate::new(key))
    }
}

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(attributes::plugin);
}
