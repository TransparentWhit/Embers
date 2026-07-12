use super::attributes::{AttributeModification, AttributeModifier, Attributes, DamageTaken};
use super::living_actor;
use crate::pld::foundry::{GltfElementId, actor_scene};
use crate::utils::{NamespacedKey, template_bundle_for};
use avian3d::prelude::*;
use bevy::prelude::*;
use std::sync::LazyLock;

pub static KEY: LazyLock<NamespacedKey> = LazyLock::new(|| NamespacedKey::new_embers("dummy"));

pub fn dummy() -> impl Scene {
    bsn! {
        living_actor(&KEY, false)
        Collider::cuboid(1., 3., 1.)
        actor_scene(&KEY, GltfElementId::Default)
        template_bundle_for(Attributes::<DamageTaken>::new_virtual().with_modifiers([AttributeModifier::new(NamespacedKey::new_embers("e"), AttributeModification::AddMultipliedValue(-1.))]))
    }
}
