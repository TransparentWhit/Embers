use crate::utils::NamespacedKey;
use crate::world::entity::living::living_entity;
use avian3d::prelude::Collider;
use bevy::prelude::*;
use std::collections::HashMap;
use std::sync::LazyLock;

#[macro_export]
macro_rules! creeper {
    [$($extra: expr),* $(,)?] => {$crate::living_entity![
        $crate::world::entity::living::LivingEntity::new(&std::collections::HashMap::from([
            ($crate::utils::NamespacedKey::new("embers", "max_health"), 20f32),
        ])),
        bevy::prelude::Mesh3d(meshes.add(Cylinder { radius: 0.5, half_height: 0.85 }.mesh())),
        bevy::prelude::MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
        Collider::cylinder(0.5, 1.7),
        $(, $extra)*
    ]};
}

static ATTRIBUTES: LazyLock<HashMap<NamespacedKey, f32>> = LazyLock::new(|| {HashMap::from([
    (NamespacedKey::new("embers", "max_health"), 20f32),
])});
static HITBOX: LazyLock<Collider> = LazyLock::new(|| Collider::cylinder(0.5, 1.7));

#[derive(Component)]
pub struct Creeper {
    
}

pub fn creeper() -> impl Bundle {
    (
        living_entity(&ATTRIBUTES),
        //Mesh3d(),
        //MeshMaterial3d(),
        HITBOX.clone(),
        Creeper {},
    )
}
