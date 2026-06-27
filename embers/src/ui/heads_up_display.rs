use super::dim::DimensionRootNode;
use super::{ActiveOverlay, AnimatedTexture, TextureAnimation, TextureScaling};
use crate::dim::actor::living::player::{
    HOTBAR_SLOTS, Player, PlayerInventory, SelectedHotbarSlot, process_input_hotbar_in_hud,
};
use crate::dim::item::ItemStack;
use crate::dim::item::inv::InventorySlot;
use crate::pld::{PayloadManager, item_image_node, ui_image_node};
use crate::utils::Keyed;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

#[derive(Component, Debug)]
struct HotbarSlotNode(InventorySlot);

#[derive(Component)]
struct HotbarSelectionIndicatorNode;

#[derive(Component)]
struct MainHandSlotNode;

fn init(
    mut commands: Commands,
    payload_manager: Res<PayloadManager>,
    asset_server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    texture_atlas_layouts: Res<Assets<TextureAtlasLayout>>,
    texture_animations: Res<Assets<TextureAnimation>>,
    texture_scalings: Res<Assets<TextureScaling>>,
    dimension_root_node: Single<Entity, With<DimensionRootNode>>,
    mut cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    cursor_options.grab_mode = CursorGrabMode::Confined;
    commands
        .spawn((
            ChildOf(*dimension_root_node),
            DespawnOnExit(ActiveOverlay::HeadsUpDisplay),
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
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        width: px(122),
                        height: px(22),
                        margin: UiRect::horizontal(px(3)),
                        ..default()
                    },
                    ui_image_node(
                        &payload_manager,
                        &asset_server,
                        &images,
                        &texture_atlas_layouts,
                        &texture_animations,
                        &texture_scalings,
                        "hotbar",
                    ),
                ))
                .with_children(|parent| {
                    for i in 0..HOTBAR_SLOTS {
                        parent.spawn((
                            Node {
                                left: px(1),
                                top: px(1),
                                width: px(16),
                                height: px(16),
                                margin: UiRect::all(px(2)),
                                ..default()
                            },
                            ImageNode::default(),
                            AnimatedTexture::default(),
                            HotbarSlotNode(i),
                        ));
                    }
                    parent.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(-1),
                            top: px(-1),
                            width: px(24),
                            height: px(23),
                            ..default()
                        },
                        ui_image_node(
                            &payload_manager,
                            &asset_server,
                            &images,
                            &texture_atlas_layouts,
                            &texture_animations,
                            &texture_scalings,
                            "hotbar_selection",
                        ),
                        HotbarSelectionIndicatorNode,
                    ));
                });
            parent
                .spawn((
                    Node {
                        width: px(22),
                        height: px(22),
                        margin: UiRect::horizontal(px(3)),
                        ..default()
                    },
                    ui_image_node(
                        &payload_manager,
                        &asset_server,
                        &images,
                        &texture_atlas_layouts,
                        &texture_animations,
                        &texture_scalings,
                        "main_hand",
                    ),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Node {
                            left: px(1),
                            top: px(1),
                            width: px(16),
                            height: px(16),
                            margin: UiRect::all(px(2)),
                            ..default()
                        },
                        ImageNode::default(),
                        AnimatedTexture::default(),
                        MainHandSlotNode,
                    ));
                });
        });
}

fn fina(mut cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>) {
    cursor_options.grab_mode = CursorGrabMode::None;
}

fn update_hotbar_selection_indicator(
    player: Single<Ref<SelectedHotbarSlot>, With<Player>>,
    mut hotbar_selection_node: Single<&mut Node, With<HotbarSelectionIndicatorNode>>,
) {
    let ref selected_hotbar_slot = *player;
    if selected_hotbar_slot.is_changed() {
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

fn update_hotbar(
    mut commands: Commands,
    payload_manager: Res<PayloadManager>,
    asset_server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    texture_atlas_layouts: Res<Assets<TextureAtlasLayout>>,
    texture_animations: Res<Assets<TextureAnimation>>,
    texture_scalings: Res<Assets<TextureScaling>>,
    player_inventory: Single<Ref<PlayerInventory>>,
    items: Query<&ItemStack>,
    main_hand_slot: Single<Entity, With<MainHandSlotNode>>,
    mut hotbar_slots: Query<(Entity, &HotbarSlotNode), Without<MainHandSlotNode>>,
) {
    // TODO use bsn after Bevy 0.19
    if !player_inventory.is_changed() {
        return;
    }
    {
        commands
            .entity(*main_hand_slot)
            .insert(match player_inventory.main_hand() {
                Some(item) => item_image_node(
                    &payload_manager,
                    &asset_server,
                    &images,
                    &texture_atlas_layouts,
                    &texture_animations,
                    &texture_scalings,
                    items
                        .get(item)
                        .expect("Inventory held an item that doesn't exist")
                        .key(),
                ),
                None => default(),
            });
    }
    for (hotbar_entity, hotbar_slot) in hotbar_slots.iter_mut() {
        commands
            .entity(hotbar_entity)
            .insert(match player_inventory.hotbar(hotbar_slot.0) {
                Some(item) => item_image_node(
                    &payload_manager,
                    &asset_server,
                    &images,
                    &texture_atlas_layouts,
                    &texture_animations,
                    &texture_scalings,
                    items
                        .get(item)
                        .expect("Inventory held an item that doesn't exist")
                        .key(),
                ),
                None => default(),
            });
    }
}

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(ActiveOverlay::HeadsUpDisplay), init)
        .add_systems(OnExit(ActiveOverlay::HeadsUpDisplay), fina)
        .add_systems(Update, update_hotbar.after(process_input_hotbar_in_hud))
        .add_systems(Update, update_hotbar_selection_indicator);
}
