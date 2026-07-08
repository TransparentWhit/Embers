pub mod item_actor;
pub mod living;
pub mod primed_tnt;
pub mod projectile;

use super::{ActiveDimension, LoadedDimensions};
use crate::ui::GameState;
use crate::utils::NamespacedKey;
use bevy::ecs::template::TemplateContext;
use bevy::prelude::*;
use thiserror::Error;

#[derive(Component)]
#[require(Transform)]
pub struct Actor;

impl FromTemplate for Actor {
    type Template = ActorTemplate;
}

#[derive(Default)]
pub struct ActorTemplate {
    dimension: Option<NamespacedKey>,
}

#[derive(Debug, Error)]
#[error("Can not spawn entity in nonexistent dimension {dimension}")]
struct NonexistentDimensionError {
    dimension: NamespacedKey,
}

impl Template for ActorTemplate {
    type Output = Actor;
    fn build_template(&self, context: &mut TemplateContext) -> Result<Self::Output> {
        context
            .entity
            .insert(ChildOf(if let Some(dimension) = &self.dimension {
                context
                    .resource::<LoadedDimensions>()
                    .0
                    .get(dimension)
                    .copied()
                    .ok_or_else(|| NonexistentDimensionError {
                        dimension: dimension.clone(),
                    })?
            } else {
                context
                    .entity
                    .world()
                    .try_query_filtered::<Entity, With<ActiveDimension>>()
                    .unwrap()
                    .single(context.entity.world())?
            }));
        Ok(Actor)
    }
    fn clone_template(&self) -> Self {
        Self {
            dimension: self.dimension.clone(),
        }
    }
}

pub fn actor() -> impl Scene {
    bsn! {
        Actor
    }
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (primed_tnt::fuse).run_if(in_state(GameState::Dimension)),
    )
    .add_plugins(living::plugin);
}
