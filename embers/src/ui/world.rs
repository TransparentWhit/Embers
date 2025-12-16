use crate::GameState;
use crate::ui::{ScalableComponent, UIScale, scalable};
use crate::utils::Keyed;
use crate::utils::assets::GLOBAL_ASSETS;
use crate::world::entity::item_entity::{ItemEntity, item_entity};
use crate::world::entity::living::dummy::dummy;
use crate::world::entity::living::player::{
    HOTBAR_SLOTS, Player, PlayerInventory, SelectedHotbarSlot, player, process_input,
};
use crate::world::entity::tnt::tnt;
use crate::world::item::inventory::{
    InventorySlot, ItemDestination, ItemMoveQuantity, ItemSource, MoveItemCommandExt,
};
use crate::world::item::{ItemStack, sword};
use avian3d::prelude::*;
use bevy::camera::{ScalingMode, Viewport};
use bevy::input::keyboard::KeyboardInput;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowResized};
use std::ops::DerefMut;

#[derive(States, Clone, Copy, Default, Eq, PartialEq, Debug, Hash)]
enum WorldState {
    Main,
    Options,
    #[default]
    Disabled,
}

#[derive(Component)]
struct Ground;

#[derive(Component)]
pub enum PlayerCamera {
    Isometric {
        distance: f32,
        height: f32,
        /// **In radians**
        angle: f32,
    },
}

#[derive(Component)]
struct HotbarSlot(InventorySlot);

#[derive(Component)]
struct HotbarSelection;

#[derive(Component)]
struct MainHandSlot;

#[derive(Message)]
pub struct HotbarSelectionUpdated;

fn init(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        DespawnOnExit(GameState::World),
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
                    scalable(|scale| Node {
                        left: percent(50),
                        bottom: px(scale * 7),
                        margin: UiRect::left(px(scale * -185 / 2)),
                        position_type: PositionType::Absolute,
                        width: px(scale * 185),
                        height: px(scale * 18),
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    }),
                    children![
                        (
                            scalable(|scale| Node {
                                width: px(scale * 122),
                                height: px(scale * 22),
                                margin: UiRect::horizontal(px(scale * 3)),
                                ..default()
                            }),
                            GLOBAL_ASSETS.image_node(&asset_server, "hotbar"),
                            Children::spawn({
                                let mut hotbar_slots = Vec::with_capacity(HOTBAR_SLOTS as usize);
                                for i in 0..HOTBAR_SLOTS {
                                    hotbar_slots.push((
                                        scalable(if i == 0 {
                                            |scale| Node {
                                                left: px(scale * 1),
                                                top: px(scale * 1),
                                                width: px(scale * 16),
                                                height: px(scale * 16),
                                                margin: UiRect::all(px(scale * 2)),
                                                ..default()
                                            }
                                        } else {
                                            |scale| Node {
                                                top: px(scale * 1),
                                                width: px(scale * 16),
                                                height: px(scale * 16),
                                                margin: UiRect::all(px(scale * 2)),
                                                ..default()
                                            }
                                        }),
                                        ImageNode::default(),
                                        HotbarSlot(i),
                                    ));
                                }
                                (
                                    hotbar_slots,
                                    Spawn((
                                        scalable(|scale| Node {
                                            position_type: PositionType::Absolute,
                                            left: px(scale * -1),
                                            top: px(scale * -1),
                                            width: px(scale * 24),
                                            height: px(scale * 23),
                                            ..default()
                                        }),
                                        GLOBAL_ASSETS.image_node(&asset_server, "hotbar_selection"),
                                        HotbarSelection,
                                    )),
                                )
                            })
                        ),
                        (
                            scalable(|scale| Node {
                                width: px(scale * 22),
                                height: px(scale * 22),
                                margin: UiRect::horizontal(px(scale * 3)),
                                ..default()
                            }),
                            GLOBAL_ASSETS.image_node(&asset_server, "main_hand"),
                            children![(
                                scalable(|scale| Node {
                                    left: px(scale * 1),
                                    top: px(scale * 1),
                                    width: px(scale * 16),
                                    height: px(scale * 16),
                                    margin: UiRect::all(px(scale * 2)),
                                    ..default()
                                }),
                                ImageNode::default(),
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
                Ground,
                RigidBody::Static,
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
                player(),
                Transform::from_xyz(0.0, 1.0, 0.0),
                LinearVelocity::from(Vec3::new(0., 10., 0.)),
            ),
            (tnt(&asset_server), Transform::from_xyz(0.0, 0.5, 0.0)),
            (dummy(&asset_server), Transform::from_xyz(5.0, 0.5, 0.0)),
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
    main_hand_slot: Single<&mut ImageNode, With<MainHandSlot>>,
    mut hotbar_slots: Query<(&mut ImageNode, &HotbarSlot), Without<MainHandSlot>>,
) {
    if !player_inventory.is_changed() {
        return;
    }
    let player_inventory = player_inventory;
    let _: Mut<ImageNode>;
    *main_hand_slot.into_inner() = match player_inventory.main_hand() {
        Some(item) => GLOBAL_ASSETS.item_image(
            &asset_server,
            items
                .get(item)
                .expect("Inventory held an item that doesn't exist")
                .key(),
        ),
        None => ImageNode::default(),
    };
    for (hotbar_image, hotbar_slot) in hotbar_slots.iter_mut() {
        *hotbar_image.into_inner() = match player_inventory.hotbar(hotbar_slot.0) {
            Some(item) => GLOBAL_ASSETS.item_image(
                &asset_server,
                items
                    .get(item)
                    .expect("Inventory held an item that doesn't exist")
                    .key(),
            ),
            None => ImageNode::default(),
        };
    }
}

fn update_inventory(player_inventory: Single<&PlayerInventory>) {}

fn update_hotbar_selection(
    mut updated_message: MessageReader<HotbarSelectionUpdated>,
    mut hotbar_selection_node: Single<
        (&mut ScalableComponent<Node>, &mut Node),
        With<HotbarSelection>,
    >,
    selected_hotbar_slot: Single<&SelectedHotbarSlot>,
) {
    for _ in updated_message.read() {
        let (scalable, hotbar_selection_node) = hotbar_selection_node.deref_mut();
        let selected_hotbar_slot = selected_hotbar_slot.0 as UIScale;
        **scalable = ScalableComponent::dynamic(move |scale| Node {
            position_type: PositionType::Absolute,
            left: px(scale * (-1 + selected_hotbar_slot * 20)),
            top: px(scale * -1),
            width: px(scale * 24),
            height: px(scale * 23),
            ..default()
        });
        scalable.apply(hotbar_selection_node);
    }
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(
        OnEnter(GameState::World),
        (init, resize_camera, |mut commands: Commands| {
            let sword = commands.spawn(sword()).id();
            commands.spawn(item_entity(sword)).add_child(sword);
        })
            .chain(),
    );
    app.add_systems(
        Update,
        (|mut commands: Commands,
          mut player_inv: Single<(Entity, &PlayerInventory)>,
          item_entities: Query<Entity, (With<ItemEntity>, Without<PlayerInventory>)>| {
            let (inv_entity, inv) = player_inv.deref_mut();
            for item_entity in item_entities.iter() {
                commands.move_item(
                    ItemSource::item_entity(item_entity),
                    ItemDestination::inventory_range(*inv_entity, 0..3, inv),
                    ItemMoveQuantity::All,
                );
            }
        })
        .run_if(on_message::<KeyboardInput>),
    );
    app.add_systems(Update, resize_camera.run_if(on_message::<WindowResized>));
    app.add_systems(Update, update_player_camera);
    app.add_systems(Update, update_hotbar.after(process_input));
    app.add_message::<HotbarSelectionUpdated>();
    app.add_systems(Update, update_hotbar_selection);
}
