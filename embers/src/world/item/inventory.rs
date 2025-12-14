use crate::registry::DynamicRegistry;
use crate::utils::Marker;
use crate::world::entity::item_entity::ItemEntity;
use crate::world::item::{ItemComponent, ItemStack, MaxStackSize, StackCount};
use bevy::ecs::system::entity_command::log_components;
use bevy::prelude::*;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut, Range};

pub type InventorySlot = i8;

#[derive(Component)]
pub struct Inventory<const N: usize, M: Marker = ()>([Option<Entity>; N], PhantomData<M>);

impl<const N: usize, M: Marker> Inventory<N, M> {
    pub fn new() -> Self {
        Self([const { None }; N], PhantomData)
    }
    pub fn slice(&self) -> &[Option<Entity>] {
        self.0.as_slice()
    }
    pub fn slice_mut(&mut self) -> &mut [Option<Entity>] {
        self.0.as_mut_slice()
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

impl<const N: usize, M: Marker> Index<Range<InventorySlot>> for Inventory<N, M> {
    type Output = [Option<Entity>];
    fn index(&self, index: Range<InventorySlot>) -> &Self::Output {
        &self.0[index.start as usize..index.end as usize]
    }
}

impl<const N: usize, M: Marker> IndexMut<Range<InventorySlot>> for Inventory<N, M> {
    fn index_mut(&mut self, index: Range<InventorySlot>) -> &mut Self::Output {
        &mut self.0[index.start as usize..index.end as usize]
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
    if !source_ref.contains::<ItemStack>() || !target_ref.contains::<ItemStack>() {
        return ItemStackResult::NotStackable;
    }
    if world
        .resource::<DynamicRegistry<dyn ItemComponent>>()
        .iter()
        .all(|item_component| item_component.can_stack(source_ref, target_ref))
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
        source_ref
            .get::<MaxStackSize>()
            .cloned()
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

#[derive(Clone, Copy)]
pub enum ItemSource<const N: usize, M: Marker> {
    // Range isn't Copy
    InventorySlice(Entity, (InventorySlot, InventorySlot), PhantomData<M>),
    InventorySlot(Entity, InventorySlot, PhantomData<M>),
    ItemEntity(Entity),
}

impl ItemSource<0, ()> {
    pub fn item_entity(item_entity: Entity) -> Self {
        Self::ItemEntity(item_entity)
    }
}

impl<const N: usize, M: Marker> ItemSource<N, M> {
    pub fn inventory_range(
        inventory: Entity,
        range: Range<InventorySlot>,
        _: &Inventory<N, M>,
    ) -> Self {
        Self::InventorySlice(inventory, (range.start, range.end), PhantomData)
    }
    pub fn inventory_slot(inventory: Entity, slot: InventorySlot, _: &Inventory<N, M>) -> Self {
        Self::InventorySlot(inventory, slot, PhantomData)
    }
    fn verify_existence(&self, world: &World) -> bool {
        world
            .get_entity(*match self {
                Self::InventorySlice(inventory, _, _) => inventory,
                Self::InventorySlot(inventory, _, _) => inventory,
                Self::ItemEntity(item_entity) => item_entity,
            })
            .is_ok()
    }
    fn single(&self) -> SingleItemSource<N, M> {
        match self {
            Self::InventorySlice(..) => {
                panic!("An inventory slice can not be interpreted as a single item slot")
            }
            &Self::InventorySlot(inventory, slot, _) => {
                SingleItemSource::InventorySlot(inventory, slot, PhantomData)
            }
            &Self::ItemEntity(item_entity) => SingleItemSource::ItemEntity(item_entity),
        }
    }
}

#[derive(Clone, Copy)]
pub enum ItemDestination<const N: usize, M: Marker> {
    // Range isn't Copy
    InventorySlice(Entity, (InventorySlot, InventorySlot), PhantomData<M>),
    InventorySlot(Entity, InventorySlot, PhantomData<M>),
    ItemEntity(Entity),
}

impl ItemDestination<0, ()> {
    pub fn item_entity(item_entity: Entity) -> Self {
        Self::ItemEntity(item_entity)
    }
}

impl<const N: usize, M: Marker> ItemDestination<N, M> {
    pub fn inventory_range(
        inventory: Entity,
        range: Range<InventorySlot>,
        _: &Inventory<N, M>,
    ) -> Self {
        Self::InventorySlice(inventory, (range.start, range.end), PhantomData)
    }
    pub fn inventory_slot(inventory: Entity, slot: InventorySlot, _: &Inventory<N, M>) -> Self {
        Self::InventorySlot(inventory, slot, PhantomData)
    }
    fn verify_existence(&self, world: &World) -> bool {
        world
            .get_entity(*match self {
                Self::InventorySlice(inventory, _, _) => inventory,
                Self::InventorySlot(inventory, _, _) => inventory,
                Self::ItemEntity(item_entity) => item_entity,
            })
            .is_ok()
    }
    fn single(&self) -> SingleItemDestination<N, M> {
        match self {
            Self::InventorySlice(..) => {
                panic!("An inventory slice can not be interpreted as a single item slot")
            }
            &Self::InventorySlot(inventory, slot, _) => {
                SingleItemDestination::InventorySlot(inventory, slot, PhantomData)
            }
            &Self::ItemEntity(item_entity) => SingleItemDestination::ItemEntity(item_entity),
        }
    }
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub enum ItemMoveQuantity {
    All,
    Half,
    One,
}

enum SingleItemSource<const N: usize, M: Marker> {
    InventorySlot(Entity, InventorySlot, PhantomData<M>),
    ItemEntity(Entity),
}

impl<const N: usize, M: Marker> SingleItemSource<N, M> {
    fn set_item(&mut self, world: &mut World, item: Entity) {
        match self {
            Self::InventorySlot(inventory, slot, _) => {
                world.get_mut::<Inventory<N, M>>(*inventory).unwrap()[*slot] = Some(item)
            }
            Self::ItemEntity(item_entity) => {
                world.get_mut::<ItemEntity>(*item_entity).unwrap().0 = item
            }
        };
    }
    /// Call this AFTER the relationship between the owner and the item has been removed
    fn drop_slot(&mut self, world: &mut World) {
        match self {
            Self::InventorySlot(inventory, slot, _) => {
                world.get_mut::<Inventory<N, M>>(*inventory).unwrap()[*slot] = None
            }
            Self::ItemEntity(item_entity) => world.entity_mut(*item_entity).despawn(),
        };
    }
}

enum SingleItemDestination<const N: usize, M: Marker> {
    InventorySlot(Entity, InventorySlot, PhantomData<M>),
    ItemEntity(Entity),
}

impl<const N: usize, M: Marker> SingleItemDestination<N, M> {
    fn set_item(&mut self, world: &mut World, item: Entity) {
        match self {
            Self::InventorySlot(inventory, slot, _) => {
                log_components().apply(world.entity_mut(*inventory));
                world.get_mut::<Inventory<N, M>>(*inventory).unwrap()[*slot] = Some(item)
            }
            Self::ItemEntity(item_entity) => {
                world.get_mut::<ItemEntity>(*item_entity).unwrap().0 = item
            }
        };
    }
}

pub struct MoveItemCommand<
    const N_SOURCE: usize,
    MSource: Marker,
    const N_DESTINATION: usize,
    MDestination: Marker,
> {
    source: ItemSource<N_SOURCE, MSource>,
    destination: ItemDestination<N_DESTINATION, MDestination>,
    quantity: ItemMoveQuantity,
}

pub trait MoveItemCommandExt {
    fn move_item<
        const N_SOURCE: usize,
        MSource: Marker,
        const N_DESTINATION: usize,
        MDestination: Marker,
    >(
        &mut self,
        source: ItemSource<N_SOURCE, MSource>,
        destination: ItemDestination<N_DESTINATION, MDestination>,
        quantity: ItemMoveQuantity,
    );
}

impl<'w, 's> MoveItemCommandExt for Commands<'w, 's> {
    fn move_item<
        const N_SOURCE: usize,
        MSource: Marker,
        const N_DESTINATION: usize,
        MDestination: Marker,
    >(
        &mut self,
        source: ItemSource<N_SOURCE, MSource>,
        destination: ItemDestination<N_DESTINATION, MDestination>,
        quantity: ItemMoveQuantity,
    ) {
        self.queue(MoveItemCommand {
            source,
            destination,
            quantity,
        });
    }
}

impl<const N_SOURCE: usize, MSource: Marker, const N_DESTINATION: usize, MDestination: Marker>
    Command for MoveItemCommand<N_SOURCE, MSource, N_DESTINATION, MDestination>
{
    fn apply(self, world: &mut World) {
        if !self.source.verify_existence(world) || !self.destination.verify_existence(world) {
            return;
        }
        let mut move_single =
            |source: &mut SingleItemSource<N_SOURCE, MSource>,
             destination: &mut SingleItemDestination<N_DESTINATION, MDestination>,
             swap_if_not_stackable: bool| {
                let (src_owner, src_item) = match *source {
                    SingleItemSource::InventorySlot(inventory, slot, _) => (
                        inventory,
                        match world
                            .get_mut::<Inventory<N_SOURCE, MSource>>(inventory)
                            .unwrap()[slot]
                        {
                            Some(item) => item,
                            None => return,
                        },
                    ),
                    SingleItemSource::ItemEntity(item_entity) => (
                        item_entity,
                        world.get_mut::<ItemEntity>(item_entity).unwrap().0,
                    ),
                };
                let (dst_owner, dst_slot) = match *destination {
                    SingleItemDestination::InventorySlot(inventory, slot, _) => (
                        inventory,
                        world
                            .get_mut::<Inventory<N_DESTINATION, MDestination>>(inventory)
                            .unwrap()[slot],
                    ),
                    SingleItemDestination::ItemEntity(item_entity) => (
                        item_entity,
                        Some(world.get_mut::<ItemEntity>(item_entity).unwrap().0),
                    ),
                };
                match dst_slot {
                    Some(dst_item) => match try_stack(world, src_item, dst_item, self.quantity) {
                        ItemStackResult::NotStackable => {
                            if swap_if_not_stackable {
                                world
                                    .entity_mut(src_item)
                                    .remove_related::<ChildOf>(&[src_owner])
                                    .add_one_related::<ChildOf>(dst_owner);
                                world
                                    .entity_mut(dst_item)
                                    .remove_related::<ChildOf>(&[dst_owner])
                                    .add_one_related::<ChildOf>(src_owner);
                                source.set_item(world, dst_item);
                                destination.set_item(world, src_item);
                            }
                        }
                        ItemStackResult::Remaining => {}
                        ItemStackResult::Consumed => source.drop_slot(world),
                    },
                    None => {
                        world
                            .entity_mut(src_item)
                            .remove_related::<ChildOf>(&[src_owner])
                            .add_one_related::<ChildOf>(dst_owner);
                        source.drop_slot(world);
                        destination.set_item(world, src_item);
                    }
                }
            };
        match (&self.source, &self.destination) {
            (ItemSource::InventorySlice(..), ItemDestination::InventorySlice(..)) => {
                unimplemented!("Can not move from an inventory slice to another inventory slice")
            }
            (&ItemSource::InventorySlice(src_inventory, ref src_range, _), _) => {
                let mut destination = self.destination.single();
                for src_slot in src_range.0..src_range.1 {
                    move_single(
                        &mut SingleItemSource::InventorySlot(src_inventory, src_slot, PhantomData),
                        &mut destination,
                        false,
                    );
                }
            }
            (_, &ItemDestination::InventorySlice(dst_inventory, ref dst_range, _)) => {
                let mut source = self.source.single();
                for dst_slot in dst_range.0..dst_range.1 {
                    move_single(
                        &mut source,
                        &mut SingleItemDestination::InventorySlot(
                            dst_inventory,
                            dst_slot,
                            PhantomData,
                        ),
                        false,
                    );
                }
            }
            _ => move_single(
                &mut self.source.single(),
                &mut self.destination.single(),
                true,
            ),
        };
    }
}
