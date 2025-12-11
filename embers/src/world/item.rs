use crate::utils::NamespacedKey;
use bevy::prelude::*;
use embers_macros::identify;
use std::time::Duration;

pub mod embers {
    macro_rules! item {
        ($id: ident, $key: expr) => {
            pub static $id: std::sync::LazyLock<$crate::utils::NamespacedKey> =
                std::sync::LazyLock::new(|| $crate::utils::NamespacedKey::new_embers($key));
        };
    }
    item!(SWORD, "sword");
}

pub type InventorySlot = i8;

#[derive(Component)]
pub struct Inventory<const N: usize>([Option<Entity>; N]);

impl<const N: usize> Default for Inventory<N> {
    fn default() -> Self {
        Self([const { None }; N])
    }
}

impl<const N: usize> Inventory<N> {
    pub fn new() -> Self {
        Default::default()
    }
}

enum ItemStackResult {
    /// Failure; Cannot stack.
    NotStackable,
    /// Success; `source` is not empty.
    Remaining,
    /// Success; `source` is empty.
    Consumed,
}

impl ItemStackResult {
    fn failure(&self) -> bool {
        matches!(self, Self::NotStackable)
    }
    fn success(&self) -> bool {
        matches!(self, Self::Remaining | Self::Consumed)
    }
    fn should_drop(&self) -> bool {
        matches!(self, Self::Consumed)
    }
}

fn try_stack(
    world: &World,
    commands: &mut Commands,
    source: Entity,
    target: Entity,
    quantity: ItemMoveQuantity,
) -> ItemStackResult {
    if !world.entity(source).contains::<ItemStack>()
        || !world.entity(target).contains::<ItemStack>()
    {
        return ItemStackResult::NotStackable;
    }
    if true
    /*todo*/
    {
        return ItemStackResult::NotStackable;
    }
    let source_ref = world.entity(source);
    let source_count = source_ref.get::<StackCount>().unwrap().0;
    let target_count = world.entity(target).get::<StackCount>().unwrap().0;
    let amount_to_transfer = match quantity {
        ItemMoveQuantity::All => source_count,
        ItemMoveQuantity::Half => source_count.div_ceil(2),
        ItemMoveQuantity::One => 1,
    }
    .min(
        source_ref
            .get::<MaxStackSize>()
            .cloned()
            .unwrap_or_default()
            .0
            .saturating_sub(target_count),
    );
    commands
        .entity(target)
        .insert(StackCount(target_count + amount_to_transfer));
    let new_source_count = source_count - amount_to_transfer;
    let source_destroyed = new_source_count == 0;
    if source_destroyed {
        commands.entity(source).despawn();
        ItemStackResult::Consumed
    } else {
        commands.entity(source).insert(StackCount(new_source_count));
        ItemStackResult::Remaining
    }
}

pub enum ItemSource<'a> {
    InventorySlice(Entity, &'a mut [Option<Entity>]),
    InventorySlot(Entity, &'a mut Option<Entity>),
    ItemEntity(Entity, &'a mut Entity),
}

impl<'a> ItemSource<'a> {
    fn single(self) -> SingleItemSource<'a> {
        match self {
            Self::InventorySlice(..) => {
                panic!("An inventory slice can not be interpreted as a single item slot")
            }
            Self::InventorySlot(inventory, slot) => {
                SingleItemSource::InventorySlot(inventory, slot)
            }
            Self::ItemEntity(item_entity, item) => SingleItemSource::ItemEntity(item_entity, item),
        }
    }
}

enum SingleItemSource<'a> {
    InventorySlot(Entity, &'a mut Option<Entity>),
    ItemEntity(Entity, &'a mut Entity),
}

impl SingleItemSource<'_> {
    fn set_item(&mut self, item: Entity) {
        match self {
            Self::InventorySlot(_, slot) => **slot = Some(item),
            Self::ItemEntity(_, entity_item) => **entity_item = item,
        };
    }
    /// Call this AFTER the relationship between the owner and the item has been removed
    fn drop_slot(&mut self, commands: &mut Commands) {
        match self {
            Self::InventorySlot(_, slot) => **slot = None,
            Self::ItemEntity(item_entity, _) => commands.entity(*item_entity).despawn(),
        };
    }
}

pub enum ItemDestination<'a> {
    InventorySlice(Entity, &'a mut [Option<Entity>]),
    InventorySlot(Entity, &'a mut Option<Entity>),
    ItemEntity(Entity, &'a mut Entity),
}

impl<'a> ItemDestination<'a> {
    fn single(self) -> SingleItemDestination<'a> {
        match self {
            Self::InventorySlice(..) => {
                panic!("An inventory slice can not be interpreted as a single item slot")
            }
            Self::InventorySlot(inventory, slot) => {
                SingleItemDestination::InventorySlot(inventory, slot)
            }
            Self::ItemEntity(item_entity, item) => {
                SingleItemDestination::ItemEntity(item_entity, item)
            }
        }
    }
}

enum SingleItemDestination<'a> {
    InventorySlot(Entity, &'a mut Option<Entity>),
    ItemEntity(Entity, &'a mut Entity),
}

impl SingleItemDestination<'_> {
    fn set_item(&mut self, item: Entity) {
        match self {
            Self::InventorySlot(_, slot) => **slot = Some(item),
            Self::ItemEntity(_, entity_item) => **entity_item = item,
        };
    }
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub enum ItemMoveQuantity {
    All,
    Half,
    One,
}

enum NonStackableOperation {
    Ignore,
    Swap,
}

fn move_item(
    world: &World,
    commands: &mut Commands,
    source: ItemSource,
    destination: ItemDestination,
    quantity: ItemMoveQuantity,
) {
    match destination {
        ItemDestination::InventorySlice(dst_inventory, dst_slice) => {
            let mut source = source.single();
            for dst_slot in dst_slice.iter_mut() {
                move_item_internal(
                    world,
                    commands,
                    &mut source,
                    &mut SingleItemDestination::InventorySlot(dst_inventory, dst_slot),
                    quantity,
                    false,
                );
            }
        }
        _ => match source {
            ItemSource::InventorySlice(src_inventory, src_slice) => {
                let mut destination = destination.single();
                for src_slot in src_slice.iter_mut() {
                    move_item_internal(
                        world,
                        commands,
                        &mut SingleItemSource::InventorySlot(src_inventory, src_slot),
                        &mut destination,
                        quantity,
                        false,
                    );
                }
            }
            _ => move_item_internal(
                world,
                commands,
                &mut source.single(),
                &mut destination.single(),
                quantity,
                true,
            ),
        },
    }
}

fn move_item_internal(
    world: &World,
    commands: &mut Commands,
    source: &mut SingleItemSource,
    destination: &mut SingleItemDestination,
    quantity: ItemMoveQuantity,
    swap_if_not_stackable: bool,
) {
    let (src_owner, src_item) = match source {
        &mut SingleItemSource::InventorySlot(ref owner, &mut Some(ref item))
        | &mut SingleItemSource::ItemEntity(ref owner, &mut ref item) => (*owner, *item),
        SingleItemSource::InventorySlot(_, None) => return,
    };
    let (dst_owner, dst_slot) = match destination {
        SingleItemDestination::InventorySlot(inventory, slot) => (*inventory, **slot),
        SingleItemDestination::ItemEntity(item_entity, item) => (*item_entity, Some(**item)),
    };
    match dst_slot {
        Some(dst_item) => match try_stack(world, commands, src_item, dst_item, quantity) {
            ItemStackResult::NotStackable => {
                if swap_if_not_stackable {
                    commands
                        .entity(src_item)
                        .remove_related::<ChildOf>(&[src_owner])
                        .add_one_related::<ChildOf>(dst_owner);
                    commands
                        .entity(dst_item)
                        .remove_related::<ChildOf>(&[dst_owner])
                        .add_one_related::<ChildOf>(src_owner);
                    source.set_item(dst_item);
                    destination.set_item(src_item);
                }
            }
            ItemStackResult::Remaining => {}
            ItemStackResult::Consumed => source.drop_slot(commands),
        },
        None => {
            commands
                .entity(src_item)
                .remove_related::<ChildOf>(&[src_owner])
                .add_one_related::<ChildOf>(dst_owner);
            source.drop_slot(commands);
            destination.set_item(src_item);
        }
    }
}

#[derive(Component, Clone, Eq, Hash, PartialEq)]
#[require(StackCount)]
pub struct ItemStack(NamespacedKey);

#[derive(Component, Clone, Eq, Hash, PartialEq)]
pub struct StackCount(u8);

impl Default for StackCount {
    fn default() -> Self {
        Self(1)
    }
}

#[derive(Component, Clone, Eq, Hash, PartialEq)]
pub struct RangedAmmo();

#[derive(Component, Clone, Eq, Hash, PartialEq)]
pub struct Enchantments();

impl Default for Enchantments {
    fn default() -> Self {
        Self()
    }
}

#[derive(Component, Clone, Eq, Hash, PartialEq)]
pub struct MaxStackSize(u8);

impl Default for MaxStackSize {
    fn default() -> Self {
        Self(1)
    }
}

#[derive(Clone, Copy, Default, Eq, Hash, PartialEq)]
pub enum ItemActionTrigger {
    #[default]
    Click,
    DoubleClick,
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub enum ItemActionSlot {
    Armor,
    Hands(HandActionWield),
}

impl Default for ItemActionSlot {
    fn default() -> Self {
        Self::Hands(HandActionWield::default())
    }
}

#[derive(Clone, Copy, Default, Eq, Hash, PartialEq)]
pub enum HandActionWield {
    #[default]
    Single,
    Dual,
}

#[derive(Component, Clone)]
#[identify(key)]
pub struct ItemAction {
    key: NamespacedKey,
    pub on_begin: fn(),
    pub on_end: fn(),
    pub trigger: ItemActionTrigger,
    pub slot: ItemActionSlot,
    pub duration: Duration,
}

#[derive(Component)]
pub struct Weight(f32);

pub fn sword() -> impl Bundle {
    (ItemStack(embers::SWORD.clone()), Enchantments::default())
}
