#[macro_export]
macro_rules! projectile {
    [$($extra: expr),* $(,)?] => {$crate::entity![
        avian3d::dynamics::rigid_body::RigidBody::Dynamic
        $(, $extra)*
    ]};
}
