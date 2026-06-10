use crate::dim::actor::item_actor::ItemActor;
use crate::dim::item::{ItemComponent, ItemStack, MaxStackSize, StackCount};
use crate::reg::{OrRegistry, Registry, RegistryBoxed};
use crate::utils::Marker;
use bevy::prelude::*;
use std::any::type_name;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive};
use std::range::Range;
use thiserror::Error;

pub type InventorySlot = i8;

#[derive(Component, Debug)]
pub struct Inventory<const N: usize, M: Marker>([Option<Entity>; N], PhantomData<M>);

impl<const N: usize, M: Marker> Default for Inventory<N, M> {
    fn default() -> Self {
        Self([const { None }; N], PhantomData)
    }
}

impl<const N: usize, M: Marker> Inventory<N, M> {
    pub fn new() -> Self {
        default()
    }
}

impl<const N: usize, M: Marker> Index<InventorySlot> for Inventory<N, M> {
    type Output = Option<Entity>;
    fn index(&self, index: InventorySlot) -> &Self::Output {
        &self.0[index as usize]
    }
}

impl<const N: usize, M: Marker> IndexMut<InventorySlot> for Inventory<N, M> {
    fn index_mut(&mut self, index: InventorySlot) -> &mut Self::Output {
        &mut self.0[index as usize]
    }
}

macro_rules! range_index_inventory {
    ($range:ty, $pattern:pat, $slice:expr) => {
        impl<const N: usize, M: Marker> Index<$range> for Inventory<N, M> {
            type Output = [Option<Entity>];
            fn index(&self, index: $range) -> &Self::Output {
                let $pattern = index;
                &self.0[$slice]
            }
        }
        impl<const N: usize, M: Marker> IndexMut<$range> for Inventory<N, M> {
            fn index_mut(&mut self, index: $range) -> &mut Self::Output {
                let $pattern = index;
                &mut self.0[$slice]
            }
        }
    };
}

range_index_inventory!(RangeFull, _idx, ..);
range_index_inventory!(RangeFrom<InventorySlot>, idx, idx.start as usize..);
range_index_inventory!(RangeTo<InventorySlot>, idx, ..idx.end as usize);
range_index_inventory!(RangeToInclusive<InventorySlot>, idx, ..idx.end as usize);
range_index_inventory!(
    std::ops::Range<InventorySlot>,
    idx,
    idx.start as usize..idx.end as usize
);
range_index_inventory!(
    RangeInclusive<InventorySlot>,
    idx,
    *idx.start() as usize..*idx.end() as usize
);

impl<const N: usize, M: Marker> Inventory<N, M> {
    #[inline]
    pub fn size(&self) -> InventorySlot {
        N as InventorySlot
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

#[inline]
fn try_stack(
    world: &mut World,
    source: Entity,
    target: Entity,
    quantity: ItemMoveQuantity,
) -> ItemStackResult {
    let source_ref = world.entity(source);
    let target_ref = world.entity(target);
    let item_key = match source_ref
        .get::<ItemStack>()
        .zip(target_ref.get::<ItemStack>())
    {
        Some((source, target)) => {
            if source == target {
                &source.0
            } else {
                return ItemStackResult::NotStackable;
            }
        }
        None => return ItemStackResult::NotStackable,
    };
    if world
        .resource::<RegistryBoxed<dyn ItemComponent>>()
        .values()
        .any(|item_component| item_component.ne(source_ref, target_ref))
    {
        return ItemStackResult::NotStackable;
    }
    let source_count = source_ref.get::<StackCount>().unwrap().0;
    let target_count = target_ref.get::<StackCount>().unwrap().0;
    let amount_to_transfer = match quantity {
        ItemMoveQuantity::All => source_count,
        ItemMoveQuantity::Half => source_count.div_ceil(2),
        ItemMoveQuantity::One => 1,
    }
    .min(
        target_ref
            .get::<MaxStackSize>()
            .cloned()
            .or_registry(world.resource::<Registry<MaxStackSize>>(), item_key)
            .unwrap_or_default()
            .0
            .saturating_sub(target_count),
    );
    world
        .entity_mut(target)
        .insert(StackCount(target_count + amount_to_transfer));
    let new_source_count = source_count - amount_to_transfer;
    if new_source_count == 0 {
        world.entity_mut(source).despawn();
        ItemStackResult::Consumed
    } else {
        world
            .entity_mut(source)
            .insert(StackCount(new_source_count));
        ItemStackResult::Remaining
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ItemSource<const N: usize, M: Marker> {
    InventoryRange(Entity, Range<InventorySlot>, PhantomData<M>),
    InventorySlot(Entity, InventorySlot, PhantomData<M>),
    ItemActor(Entity),
}

impl ItemSource<0, ()> {
    pub fn item_actor(item_actor: Entity) -> Self {
        Self::ItemActor(item_actor)
    }
}

impl<const N: usize, M: Marker> ItemSource<N, M> {
    pub fn inventory_range(
        inventory: Entity,
        range: impl Into<Range<InventorySlot>>,
        _: &Inventory<N, M>,
    ) -> Self {
        Self::InventoryRange(inventory, range.into(), PhantomData)
    }
    pub fn inventory_slot(inventory: Entity, slot: InventorySlot, _: &Inventory<N, M>) -> Self {
        Self::InventorySlot(inventory, slot, PhantomData)
    }
    fn verify_existence(&self, world: &World) -> Result<(), InvalidItemHolderError> {
        let entity = *match self {
            Self::InventoryRange(inventory, _range, _) => inventory,
            Self::InventorySlot(inventory, _slot, _) => inventory,
            Self::ItemActor(item_actor) => item_actor,
        };
        let entity_ref = world
            .get_entity(entity)
            .map_err(|_err| InvalidItemHolderError::NonexistentEntity(entity))?;
        match self {
            Self::InventoryRange(..) | Self::InventorySlot(..) => {
                if !entity_ref.contains::<Inventory<N, M>>() {
                    return Err(InvalidItemHolderError::NotAnInventory(
                        entity,
                        N,
                        type_name::<M>(),
                    ));
                }
            }
            Self::ItemActor(..) => {
                if !entity_ref.contains::<ItemActor>() {
                    return Err(InvalidItemHolderError::NotAnItemActor(entity));
                }
            }
        }
        Ok(())
    }
    fn single(&self) -> SingleItemSource<N, M> {
        match *self {
            Self::InventoryRange(..) => {
                panic!("An inventory range can not be interpreted as a single item slot")
            }
            Self::InventorySlot(inventory, slot, _) => {
                SingleItemSource::InventorySlot(inventory, slot, PhantomData)
            }
            Self::ItemActor(item_actor) => SingleItemSource::ItemActor(item_actor),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ItemDestination<const N: usize, M: Marker> {
    InventoryRange(Entity, Range<InventorySlot>, PhantomData<M>),
    InventorySlot(Entity, InventorySlot, PhantomData<M>),
    ItemActor(Entity),
}

impl ItemDestination<0, ()> {
    pub fn item_actor(item_actor: Entity) -> Self {
        Self::ItemActor(item_actor)
    }
}

impl<const N: usize, M: Marker> ItemDestination<N, M> {
    pub fn inventory_range(
        inventory: Entity,
        range: impl Into<Range<InventorySlot>>,
        _: &Inventory<N, M>,
    ) -> Self {
        Self::InventoryRange(inventory, range.into(), PhantomData)
    }
    pub fn inventory_slot(inventory: Entity, slot: InventorySlot, _: &Inventory<N, M>) -> Self {
        Self::InventorySlot(inventory, slot, PhantomData)
    }
    fn verify_existence(&self, world: &World) -> Result<(), InvalidItemHolderError> {
        let entity = *match self {
            Self::InventoryRange(inventory, _range, _) => inventory,
            Self::InventorySlot(inventory, _slot, _) => inventory,
            Self::ItemActor(item_actor) => item_actor,
        };
        let entity_ref = world
            .get_entity(entity)
            .map_err(|_err| InvalidItemHolderError::NonexistentEntity(entity))?;
        match self {
            Self::InventoryRange(..) | Self::InventorySlot(..) => {
                if !entity_ref.contains::<Inventory<N, M>>() {
                    return Err(InvalidItemHolderError::NotAnInventory(
                        entity,
                        N,
                        type_name::<M>(),
                    ));
                }
            }
            Self::ItemActor(..) => {
                if !entity_ref.contains::<ItemActor>() {
                    return Err(InvalidItemHolderError::NotAnItemActor(entity));
                }
            }
        }
        Ok(())
    }
    fn single(&self) -> SingleItemDestination<N, M> {
        match *self {
            Self::InventoryRange(..) => {
                panic!("An inventory range can not be interpreted as a single item slot")
            }
            Self::InventorySlot(inventory, slot, _) => {
                SingleItemDestination::InventorySlot(inventory, slot, PhantomData)
            }
            Self::ItemActor(item_actor) => SingleItemDestination::ItemActor(item_actor),
        }
    }
}

#[derive(Debug, Error)]
enum InvalidItemHolderError {
    #[error("The specified entity ({0}) does not exist")]
    NonexistentEntity(Entity),
    #[error("The specified entity ({0}) doesn't contain an inventory<{1}, {2}> component")]
    NotAnInventory(Entity, usize, &'static str),
    #[error("The specified entity ({0}) doesn't contain an item actor component")]
    NotAnItemActor(Entity),
    #[error("Illegal range")]
    IllegalRange,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ItemMoveQuantity {
    All,
    Half,
    One,
}

enum SingleItemSource<const N: usize, M: Marker> {
    InventorySlot(Entity, InventorySlot, PhantomData<M>),
    ItemActor(Entity),
}

impl<const N: usize, M: Marker> SingleItemSource<N, M> {
    fn set_item(&self, world: &mut World, item: Entity) {
        match *self {
            Self::InventorySlot(inventory, slot, _) => {
                world.get_mut::<Inventory<N, M>>(inventory).unwrap()[slot] = Some(item)
            }
            Self::ItemActor(item_actor) => world.get_mut::<ItemActor>(item_actor).unwrap().0 = item,
        };
    }
    /// Call this AFTER the relationship between the owner and the item has been removed
    fn drop_slot(&self, world: &mut World) {
        match *self {
            Self::InventorySlot(inventory, slot, _) => {
                world.get_mut::<Inventory<N, M>>(inventory).unwrap()[slot] = None
            }
            Self::ItemActor(item_actor) => world.entity_mut(item_actor).despawn(),
        };
    }
}

enum SingleItemDestination<const N: usize, M: Marker> {
    InventorySlot(Entity, InventorySlot, PhantomData<M>),
    ItemActor(Entity),
}

impl<const N: usize, M: Marker> SingleItemDestination<N, M> {
    fn set_item(&self, world: &mut World, item: Entity) {
        match *self {
            Self::InventorySlot(inventory, slot, _) => {
                world.get_mut::<Inventory<N, M>>(inventory).unwrap()[slot] = Some(item)
            }
            Self::ItemActor(item_actor) => world.get_mut::<ItemActor>(item_actor).unwrap().0 = item,
        };
    }
    /// Call this AFTER the relationship between the owner and the item has been removed
    fn drop_slot(&self, world: &mut World) {
        match *self {
            Self::InventorySlot(inventory, slot, _) => {
                world.get_mut::<Inventory<N, M>>(inventory).unwrap()[slot] = None
            }
            Self::ItemActor(item_actor) => world.entity_mut(item_actor).despawn(),
        };
    }
}

pub struct MoveItemCommand<const SRC_N: usize, SrcM: Marker, const DST_N: usize, DstM: Marker> {
    source: ItemSource<SRC_N, SrcM>,
    destination: ItemDestination<DST_N, DstM>,
    quantity: ItemMoveQuantity,
}

pub trait MoveItemCommandExt {
    fn move_item<const SRC_N: usize, SrcM: Marker, const DST_N: usize, DstM: Marker>(
        &mut self,
        source: ItemSource<SRC_N, SrcM>,
        destination: ItemDestination<DST_N, DstM>,
        quantity: ItemMoveQuantity,
    );
}

impl<'world, 'state> MoveItemCommandExt for Commands<'world, 'state> {
    #[inline]
    fn move_item<const SRC_N: usize, SrcM: Marker, const DST_N: usize, DstM: Marker>(
        &mut self,
        source: ItemSource<SRC_N, SrcM>,
        destination: ItemDestination<DST_N, DstM>,
        quantity: ItemMoveQuantity,
    ) {
        self.queue(MoveItemCommand {
            source,
            destination,
            quantity,
        });
    }
}

impl<const SRC_N: usize, SrcM: Marker, const DST_N: usize, DstM: Marker> Command
    for MoveItemCommand<SRC_N, SrcM, DST_N, DstM>
{
    fn apply(self, world: &mut World) {
        if let Err(err) = self.source.verify_existence(world) {
            warn!("Could not move item from entity: {}", err);
            return;
        }
        if let Err(err) = self.destination.verify_existence(world) {
            warn!("Could not move item to entity: {}", err);
            return;
        }
        let mut move_single = |source: &SingleItemSource<SRC_N, SrcM>,
                               destination: &SingleItemDestination<DST_N, DstM>,
                               swap_if_not_stackable: bool|
         -> bool {
            let (src_owner, src_slot) = match *source {
                SingleItemSource::InventorySlot(inventory, slot, _) => (
                    inventory,
                    world.get::<Inventory<SRC_N, SrcM>>(inventory).unwrap()[slot],
                ),
                SingleItemSource::ItemActor(item_actor) => (
                    item_actor,
                    Some(world.get::<ItemActor>(item_actor).unwrap().0),
                ),
            };
            let (dst_owner, dst_slot) = match *destination {
                SingleItemDestination::InventorySlot(inventory, slot, _) => (
                    inventory,
                    world.get::<Inventory<DST_N, DstM>>(inventory).unwrap()[slot],
                ),
                SingleItemDestination::ItemActor(item_actor) => (
                    item_actor,
                    Some(world.get::<ItemActor>(item_actor).unwrap().0),
                ),
            };
            match (src_slot, dst_slot) {
                (Some(src_item), Some(dst_item)) => {
                    match try_stack(world, src_item, dst_item, self.quantity) {
                        ItemStackResult::NotStackable => {
                            if swap_if_not_stackable {
                                world.entity_mut(src_item).insert(ChildOf(dst_owner));
                                world.entity_mut(dst_item).insert(ChildOf(src_owner));
                                source.set_item(world, dst_item);
                                destination.set_item(world, src_item);
                            }
                            false
                        }
                        ItemStackResult::Remaining => false,
                        ItemStackResult::Consumed => {
                            source.drop_slot(world);
                            true
                        }
                    }
                }
                (Some(src_item), None) => {
                    world.entity_mut(src_item).insert(ChildOf(dst_owner));
                    source.drop_slot(world);
                    destination.set_item(world, src_item);
                    true
                }
                (None, Some(dst_item)) => {
                    if swap_if_not_stackable {
                        world.entity_mut(dst_item).insert(ChildOf(src_owner));
                        source.set_item(world, dst_item);
                        destination.drop_slot(world);
                    }
                    true
                }
                (None, None) => true,
            }
        };
        match (&self.source, &self.destination) {
            (ItemSource::InventoryRange(..), ItemDestination::InventoryRange(..)) => {
                unimplemented!("Can not move from an inventory range to another inventory range")
            }
            (&ItemSource::InventoryRange(src_inventory, ref src_range, _), _) => {
                let mut destination = self.destination.single();
                for src_slot in src_range.iter() {
                    move_single(
                        &mut SingleItemSource::InventorySlot(src_inventory, src_slot, PhantomData),
                        &mut destination,
                        false,
                    );
                }
            }
            (_, &ItemDestination::InventoryRange(dst_inventory, ref dst_range, _)) => {
                let mut source = self.source.single();
                for dst_slot in dst_range.iter() {
                    if move_single(
                        &mut source,
                        &mut SingleItemDestination::InventorySlot(
                            dst_inventory,
                            dst_slot,
                            PhantomData,
                        ),
                        false,
                    ) {
                        break;
                    }
                }
            }
            _ => {
                move_single(
                    &mut self.source.single(),
                    &mut self.destination.single(),
                    true,
                );
            }
        };
    }
}
