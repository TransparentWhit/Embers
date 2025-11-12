use bevy::prelude::*;

#[derive(Component)]
struct Flops(i32);

#[derive(Component)]
struct Hashes(i32);

#[derive(Component)]
struct TimeCrystals(i32);

#[derive(Component)]
pub struct Player;

fn process_input(keys: Res<ButtonInput<KeyCode>>, mouse: Res<ButtonInput<MouseButton>>) {
    mouse.pressed(MouseButton::Left);
}
