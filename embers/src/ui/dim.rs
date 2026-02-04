use crate::GameState;
use crate::dim::PhysicsPreset;
use crate::dim::actor::item_actor::item_actor_of;
use crate::dim::actor::living::AttributeBase;
use crate::dim::actor::living::dummy::dummy;
use crate::dim::actor::living::player::{
    HOTBAR_SLOTS, HotbarSelectionUpdated, Player, PlayerInventory, SelectedHotbarSlot, player,
    process_input_hotbar,
};
use crate::dim::item::inv::InventorySlot;
use crate::dim::item::{ItemStack, sword, tnt};
use crate::pld::GLOBAL_PAYLOADS;
use crate::reg::Reg;
use crate::ui::AnimatedTextureAtlas;
use crate::utils::Keyed;
use avian3d::prelude::*;
use bevy::camera::{ScalingMode, Viewport};
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowResized};
use std::ops::DerefMut;

#[derive(States, Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
enum DimensionState {
    Main,
    Options,
    #[default]
    Disabled,
}

#[derive(Component)]
struct Ground;

#[derive(Component, Debug)]
pub enum PlayerCamera {
    Isometric {
        distance: f32,
        height: f32,
        /// **In radians**
        angle: f32,
    },
}

#[derive(Component, Debug)]
struct HotbarSlot(InventorySlot);

#[derive(Component)]
struct HotbarSelection;

#[derive(Component)]
struct MainHandSlot;

fn init(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    attribute_bases: Reg<AttributeBase>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        DespawnOnExit(GameState::Dimension),
        Transform::default(),
        Node {
            width: percent(100),
            height: percent(100),
            display: Display::Flex,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        children![
            (
                Node {
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
                children![(
                    Node {
                        left: percent(50),
                        bottom: px(7),
                        margin: UiRect::left(px(-92.5)),
                        position_type: PositionType::Absolute,
                        width: px(185),
                        height: px(18),
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    children![
                        (
                            Node {
                                width: px(122),
                                height: px(22),
                                margin: UiRect::horizontal(px(3)),
                                ..default()
                            },
                            GLOBAL_PAYLOADS.ui_image(&asset_server, "hotbar"),
                            Children::spawn({
                                let mut hotbar_slots = Vec::with_capacity(HOTBAR_SLOTS as usize);
                                for i in 0..HOTBAR_SLOTS {
                                    hotbar_slots.push((
                                        Node {
                                            left: px(1),
                                            top: px(1),
                                            width: px(16),
                                            height: px(16),
                                            margin: UiRect::all(px(2)),
                                            ..default()
                                        },
                                        ImageNode::default(),
                                        AnimatedTextureAtlas::default(),
                                        HotbarSlot(i),
                                    ));
                                }
                                (
                                    hotbar_slots,
                                    Spawn((
                                        Node {
                                            position_type: PositionType::Absolute,
                                            left: px(-1),
                                            top: px(-1),
                                            width: px(24),
                                            height: px(23),
                                            ..default()
                                        },
                                        GLOBAL_PAYLOADS.ui_image(&asset_server, "hotbar_selection"),
                                        HotbarSelection,
                                    )),
                                )
                            })
                        ),
                        (
                            Node {
                                width: px(22),
                                height: px(22),
                                margin: UiRect::horizontal(px(3)),
                                ..default()
                            },
                            GLOBAL_PAYLOADS.ui_image(&asset_server, "main_hand"),
                            children![(
                                Node {
                                    left: px(1),
                                    top: px(1),
                                    width: px(16),
                                    height: px(16),
                                    margin: UiRect::all(px(2)),
                                    ..default()
                                },
                                ImageNode::default(),
                                AnimatedTextureAtlas::default(),
                                MainHandSlot,
                            ),]
                        ),
                    ],
                ),]
            ),
            (
                Camera::default(),
                Camera3d::default(),
                Bloom::default(),
                Projection::from(OrthographicProjection {
                    scaling_mode: ScalingMode::Fixed {
                        width: 16.,
                        height: 9.,
                    },
                    ..OrthographicProjection::default_3d()
                }),
                PlayerCamera::Isometric {
                    distance: 12.,
                    height: 8.,
                    angle: 35f32.to_radians(),
                },
            ),
            (
                DirectionalLight::default(),
                Transform::from_translation(Vec3::ONE).looking_at(Vec3::ZERO, Vec3::Y),
            ),
            (
                Mesh3d(meshes.add(Plane3d::default().mesh().size(20., 20.))),
                MeshMaterial3d(materials.add(Color::WHITE)),
                PhysicsPreset::Environment.physics(false),
                Ground,
                Collider::heightfield(vec![vec![0.0, 0.0], vec![0.0, 0.0]], Vec3::splat(20.)),
            ),
            (
                Mesh3d(
                    meshes.add(
                        Cylinder {
                            radius: 0.5,
                            half_height: 0.85,
                        }
                        .mesh(),
                    ),
                ),
                MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
                player(attribute_bases.as_ref()),
                Transform::from_xyz(0.0, 1.0, 0.0),
                LinearVelocity::from(Vec3::new(0., 10., 0.)),
            ),
            (
                dummy(&asset_server, attribute_bases.as_ref()),
                Transform::from_xyz(5.0, 0.5, 0.0)
            ),
            (
                item_actor_of(&asset_server, sword()),
                Transform::from_xyz(2.0, 1.0, 0.0),
            ),
            (
                item_actor_of(&asset_server, tnt()),
                Transform::from_xyz(2.0, 1.0, 0.0),
            ),
        ],
    ));
}

fn resize_camera(
    primary_window: Single<&Window, With<PrimaryWindow>>,
    mut player_camera: Single<&mut Camera, With<PlayerCamera>>,
) {
    let size = primary_window.physical_size();
    let physical_position: UVec2;
    let physical_size: UVec2;
    if size.x * 9 > size.y * 16 {
        physical_position = UVec2::new((size.x - (size.y * 16 / 9)) / 2, 0);
        physical_size = UVec2::new(size.y * 16 / 9, size.y);
    } else {
        physical_position = UVec2::new(0, (size.y - (size.x * 9 / 16)) / 2);
        physical_size = UVec2::new(size.x, size.x * 9 / 16);
    }
    player_camera.viewport = Some(Viewport {
        physical_position,
        physical_size,
        ..default()
    });
}

fn update_player_camera(
    player: Single<&Transform, With<Player>>,
    camera: Option<Single<(&mut Transform, &PlayerCamera), (With<PlayerCamera>, Without<Player>)>>,
) {
    if let Some(mut camera) = camera {
        let (camera_transform, config) = camera.deref_mut();
        match config {
            PlayerCamera::Isometric {
                distance,
                height,
                angle,
            } => {
                let player_pos = player.translation;
                camera_transform.translation =
                    player_pos + Vec3::new(distance * angle.cos(), *height, distance * angle.sin());
                camera_transform.look_at(player_pos, Vec3::Y);
            }
        }
    }
}

fn update_hotbar(
    asset_server: Res<AssetServer>,
    player_inventory: Single<Ref<PlayerInventory>>,
    items: Query<&ItemStack>,
    mut main_hand_slot: Single<(&mut ImageNode, &mut AnimatedTextureAtlas), With<MainHandSlot>>,
    mut hotbar_slots: Query<
        (&mut ImageNode, &mut AnimatedTextureAtlas, &HotbarSlot),
        Without<MainHandSlot>,
    >,
) {
    if !player_inventory.is_changed() {
        return;
    }
    {
        let (ref mut image_node, ref mut animated_texture) = *main_hand_slot;
        (**image_node, **animated_texture) = match player_inventory.main_hand() {
            Some(item) => GLOBAL_PAYLOADS.item_image(
                &asset_server,
                items
                    .get(item)
                    .expect("Inventory held an item that doesn't exist")
                    .key(),
            ),
            None => default(),
        };
    }
    for (ref mut image_node, ref mut animated_texture, hotbar_slot) in hotbar_slots.iter_mut() {
        (**image_node, **animated_texture) = match player_inventory.hotbar(hotbar_slot.0) {
            Some(item) => GLOBAL_PAYLOADS.item_image(
                &asset_server,
                items
                    .get(item)
                    .expect("Inventory held an item that doesn't exist")
                    .key(),
            ),
            None => default(),
        };
    }
}

fn update_inventory(player_inventory: Single<&PlayerInventory>) {}

fn update_hotbar_selection(
    hotbar_selection_updated_reader: MessageReader<HotbarSelectionUpdated>,
    mut hotbar_selection_node: Single<&mut Node, With<HotbarSelection>>,
    selected_hotbar_slot: Single<&SelectedHotbarSlot>,
) {
    if !hotbar_selection_updated_reader.is_empty() {
        **hotbar_selection_node = Node {
            position_type: PositionType::Absolute,
            left: px(-1 + selected_hotbar_slot.0 * 20),
            top: px(-1),
            width: px(24),
            height: px(23),
            ..default()
        };
    }
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(GameState::Dimension), (init, resize_camera).chain());
    app.add_systems(Update, resize_camera.run_if(on_message::<WindowResized>));
    app.add_systems(Update, update_player_camera);
    app.add_systems(Update, update_hotbar.after(process_input_hotbar));
    app.add_systems(Update, update_hotbar_selection);
}
