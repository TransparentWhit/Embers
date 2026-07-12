pub mod attributes;
pub mod creeper;
pub mod dummy;
pub mod player;

use super::actor;
use crate::dim::{Movements, MovementsConfig, PhysicsPreset};
use crate::pld::foundry::PayloadTemplate;
use crate::utils::{NamespacedKey, template_bundle};
use attributes::{Attributes, AttributesTemplate, DamageTaken, KnockbackTaken, MaxHealth};
use bevy::ecs::template::TemplateContext;
use bevy::prelude::*;
use bevy_tnua::builtins::TnuaBuiltinKnockback;
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
            config: PayloadTemplate::path(actor_key),
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
    bsn! {
        actor()
        { PhysicsPreset::LivingActor.physics(interactable) }
        template_bundle(AttributesTemplate::new(key.clone()))
        template_bundle(HealthTemplate)
        template(|_| Ok(TnuaController::<Movements>::default()))
        template_value(MovementConfigTemplate::new(key))
    }
}

#[derive(Message)]
pub struct Damage {
    pub target: Entity,
    pub amount: f32,
    pub knockback: DamageKnockback,
    pub source: DamageSource,
}

#[derive(Clone, Copy, PartialEq)]
pub enum DamageKnockback {
    Directional(Vec3),
    Radial(f32),
    None,
}

impl Default for DamageKnockback {
    fn default() -> Self {
        DamageKnockback::Radial(20.)
    }
}

#[derive(Clone, Copy, Default)]
pub struct DamageSource {
    pub origin: Vec3,
    pub causing_entity: Option<Entity>,
    pub direct_entity: Option<Entity>,
}

fn damage(
    mut damages: MessageReader<Damage>,
    mut commands: Commands,
    mut living_actors: Query<(
        &GlobalTransform,
        &mut Health,
        &Attributes<DamageTaken>,
        &mut TnuaController<Movements>,
        &Attributes<KnockbackTaken>,
    )>,
) {
    for Damage {
        target,
        amount,
        knockback,
        source:
            DamageSource {
                origin,
                causing_entity: _,
                direct_entity: _,
            },
    } in damages.read()
    {
        let Ok((transform, mut health, damage_taken, mut controller, knockback_taken)) =
            living_actors.get_mut(*target)
        else {
            warn!("Could not damage nonexistent living actor {}", target);
            continue;
        };
        health.0 -= damage_taken.value_for(*amount).max(0.);
        if let Some(knockback) = match *knockback {
            DamageKnockback::Directional(vector) => Some(vector),
            DamageKnockback::Radial(scalar) => {
                Some(scalar * (transform.translation() - origin).normalize_or_zero())
            }
            DamageKnockback::None => None,
        } {
            controller.action_interrupt(Movements::Knockback(TnuaBuiltinKnockback {
                shove: knockback_taken.value_for(knockback.length()).max(0.)
                    * knockback.normalize_or_zero(),
                force_forward: None,
            }))
        }
        /*commands.spawn_scene(bsn! {
            Particles3d()
        });*/
    }
}

pub(super) fn plugin(app: &mut App) {
    app.add_message::<Damage>()
        .add_systems(Update, damage)
        .add_plugins(attributes::plugin);
}
