use crate::utils::{Keyed, NamespacedKey};
use bevy::prelude::*;
use bevy::tasks::futures_lite::StreamExt;
use delegate::delegate;
use std::collections::{HashMap, HashSet, VecDeque};
use std::iter::{Empty, Map, empty, once};
use std::marker::PhantomData;
use std::mem::take;
use std::ops::Index;
use thiserror::Error;

pub trait OrRegistry {
    type Item: Send + Sync;
    fn or_registry(self, registry: &Registry<Self::Item>, key: &NamespacedKey) -> Self;
}

impl<T: Clone + Send + Sync> OrRegistry for Option<T> {
    type Item = T;
    #[inline]
    fn or_registry(self, registry: &Registry<Self::Item>, key: &NamespacedKey) -> Self {
        match &self {
            Some(..) => self,
            None => registry.get(key).cloned(),
        }
    }
}

impl<T: Clone + Send + Sync, E> OrRegistry for Result<T, E> {
    type Item = T;
    #[inline]
    fn or_registry(self, registry: &Registry<Self::Item>, key: &NamespacedKey) -> Self {
        match self {
            Ok(val) => Ok(val),
            Err(err) => registry.get(key).cloned().ok_or(err),
        }
    }
}

pub trait UnwrapOrRegistry {
    type Item: Send + Sync;
    fn unwrap_or_registry(self, registry: &Registry<Self::Item>, key: &NamespacedKey)
    -> Self::Item;
}

impl<T: Clone + Send + Sync> UnwrapOrRegistry for Option<T> {
    type Item = T;
    #[inline]
    fn unwrap_or_registry(
        self,
        registry: &Registry<Self::Item>,
        key: &NamespacedKey,
    ) -> Self::Item {
        match self {
            Some(val) => val,
            None => registry.get(key).cloned().unwrap(),
        }
    }
}

impl<T: Clone + Send + Sync, E> UnwrapOrRegistry for Result<T, E> {
    type Item = T;
    #[inline]
    fn unwrap_or_registry(
        self,
        registry: &Registry<Self::Item>,
        key: &NamespacedKey,
    ) -> Self::Item {
        match self {
            Ok(val) => val,
            Err(_err) => registry.get(key).cloned().unwrap(),
        }
    }
}

#[derive(Debug)]
pub struct RegistryCreateEvent<T> {
    _marker: PhantomData<T>,
}

pub type RegistryBoxedCreateEvent<T> = RegistryCreateEvent<Box<T>>;

#[derive(Debug)]
pub struct RegistryFreezeEvent<T> {
    _marker: PhantomData<T>,
}

pub type RegistryBoxedFreezeEvent<T> = RegistryFreezeEvent<Box<T>>;

#[derive(Debug, Message)]
pub enum RegistryEvent<T> {
    Creation(RegistryCreateEvent<T>),
    Freezing(RegistryFreezeEvent<T>),
}

impl<T> RegistryEvent<T> {
    fn new_creation() -> Self {
        Self::Creation(RegistryCreateEvent {
            _marker: PhantomData,
        })
    }
    fn new_freezing() -> Self {
        Self::Freezing(RegistryFreezeEvent {
            _marker: PhantomData,
        })
    }
}

pub type RegistryBoxedEvent<T> = RegistryEvent<Box<T>>;

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("A value with such a key already exists: {key}")]
    ValueAlreadyExists { key: NamespacedKey },
    #[error("A tag with such a key already exists: {tag}")]
    TagAlreadyExists { tag: NamespacedKey },
    #[error("This operation cannot be performed on a frozen registry")]
    RegistryFrozen {},
}

/// Represents a single item in a registry tag, which can refer to either
/// a [value](Self::Value) or [another tag](Self::Tag).
///
/// # Note
/// - Circular references will result in a panic.
/// - Tags are resolved when the registry freezes, not when the tags are registered. This means:
///   - Tags are only functional in a [frozen](Registry::is_frozen) registry.
///   - Tags can refer to items that are not yet present at the time of registration.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum RegistryTagItem {
    Value(NamespacedKey),
    Tag(NamespacedKey),
}

/// A type-safe index into a frozen registry's values, providing efficient access to regsitry values.
///
/// # Creation
/// [Registry::get_index]
///
/// # Safety
/// - **Registry Generation**: Indices are only valid in the same registry generation it was retrieved from.
///   When debug assertions are enabled, the index tracks registry generation and panics if the generation doesn't match that of the registry's.
/// - **Registry State**: Indices are only valid when the registry is frozen.
///
/// See also: [TagIndex]
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub struct ValueIndex<T> {
    index: usize,
    #[cfg(debug_assertions)]
    generation: u32,
    _marker: PhantomData<T>,
}

impl<T> ValueIndex<T> {
    fn new(index: usize, #[cfg(debug_assertions)] generation: u32) -> Self {
        Self {
            index,
            #[cfg(debug_assertions)]
            generation,
            _marker: PhantomData,
        }
    }
}

pub type BoxedValueIndex<T> = ValueIndex<Box<T>>;

/// A type-safe index into a frozen registry's tags, providing efficient access to tag membership information.
///
/// # Creation
/// [Registry::get_tag_index]
///
/// # Safety
/// - **Registry Generation**: Indices are only valid in the same registry generation it was retrieved from.
///   When debug assertions are enabled, the index tracks registry generation and panics if the generation doesn't match that of the registry's.
/// - **Registry State**: Indices are only valid when the registry is frozen.
///
/// See also: [ValueIndex]
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub struct TagIndex<T> {
    index: usize,
    #[cfg(debug_assertions)]
    generation: u32,
    _marker: PhantomData<T>,
}

impl<T> TagIndex<T> {
    fn new(index: usize, #[cfg(debug_assertions)] generation: u32) -> Self {
        Self {
            index,
            #[cfg(debug_assertions)]
            generation,
            _marker: PhantomData,
        }
    }
}

pub type BoxedTagIndex<T> = TagIndex<Box<T>>;

enum RegistryInner<T: 'static> {
    Mutating {
        values: HashMap<NamespacedKey, T>,
        tags: HashMap<NamespacedKey, HashSet<RegistryTagItem>>,
        #[cfg(debug_assertions)]
        generation: u32,
    },
    Frozen {
        indices: HashMap<NamespacedKey, usize>,
        values: Box<[T]>,
        tag_indices: HashMap<NamespacedKey, usize>,
        tags: Box<[HashSet<usize>]>,
        #[cfg(debug_assertions)]
        generation: u32,
    },
}

impl<T: 'static> RegistryInner<T> {
    fn new(#[cfg(debug_assertions)] generation: u32) -> Self {
        Self::Mutating {
            values: HashMap::new(),
            tags: HashMap::new(),
            #[cfg(debug_assertions)]
            generation,
        }
    }
    #[cfg(debug_assertions)]
    fn generation(&self) -> u32 {
        match self {
            Self::Mutating { generation, .. } => *generation,
            Self::Frozen { generation, .. } => *generation,
        }
    }
    #[inline]
    fn is_mutating(&self) -> bool {
        matches!(self, Self::Mutating { .. })
    }
    #[inline]
    fn is_frozen(&self) -> bool {
        matches!(self, Self::Frozen { .. })
    }
    fn get_index(&self, key: &NamespacedKey) -> Option<ValueIndex<T>> {
        match self {
            Self::Mutating { .. } => None,
            Self::Frozen {
                indices,
                #[cfg(debug_assertions)]
                generation,
                ..
            } => Some(ValueIndex::new(
                *indices.get(key)?,
                #[cfg(debug_assertions)]
                *generation,
            )),
        }
    }
    fn get_tag_index(&self, tag: &NamespacedKey) -> Option<TagIndex<T>> {
        match self {
            Self::Mutating { .. } => None,
            Self::Frozen {
                tag_indices,
                #[cfg(debug_assertions)]
                generation,
                ..
            } => Some(TagIndex::new(
                *tag_indices.get(tag)?,
                #[cfg(debug_assertions)]
                *generation,
            )),
        }
    }
    fn index(&self, value_index: ValueIndex<T>) -> Option<&T> {
        match self {
            Self::Mutating { .. } => None,
            Self::Frozen {
                values,
                #[cfg(debug_assertions)]
                generation,
                ..
            } => {
                #[cfg(debug_assertions)]
                if value_index.generation != *generation {
                    panic!(
                        "Value index generation mismatch (Expected {}, found {})",
                        generation, value_index.generation
                    );
                }
                values.get(value_index.index)
            }
        }
    }
    fn is_tagged_indexed(&self, tag_index: TagIndex<T>, key: &NamespacedKey) -> bool {
        match self {
            Self::Mutating { .. } => false,
            Self::Frozen {
                indices,
                tags,
                #[cfg(debug_assertions)]
                generation,
                ..
            } => {
                #[cfg(debug_assertions)]
                if tag_index.generation != *generation {
                    panic!(
                        "Tag index generation mismatch (Expected {}, found {})",
                        generation, tag_index.generation
                    );
                }
                indices.get(key).map_or(false, |value_index| {
                    tags[tag_index.index].contains(value_index)
                })
            }
        }
    }
    fn index_tagged(&self, tag_index: TagIndex<T>) -> impl Iterator<Item = &T> + ExactSizeIterator {
        enum RegistryIndexTaggedIter<'item, T: 'item, L: FnMut(&'item usize) -> &'item T> {
            Empty(Empty<&'item T>),
            MapHashSetIter(Map<std::collections::hash_set::Iter<'item, usize>, L>),
        }
        impl<'item, T: 'item, L: FnMut(&'item usize) -> &'item T> Iterator
            for RegistryIndexTaggedIter<'item, T, L>
        {
            type Item = &'item T;
            delegate! {
                to match self {
                    Self::Empty(iter) => iter,
                    Self::MapHashSetIter(iter) => iter,
                } {
                    fn next(&mut self) -> Option<Self::Item>;
                    fn size_hint(&self) -> (usize, Option<usize>);
                    fn count(self) -> usize;
                    fn fold<B, F>(self, init: B, f: F) -> B where Self: Sized, F: FnMut(B, Self::Item) -> B;
                }
            }
        }
        impl<'item, T: 'item, L: FnMut(&'item usize) -> &'item T> ExactSizeIterator
            for RegistryIndexTaggedIter<'item, T, L>
        {
            delegate! {
                to match self {
                    Self::Empty(iter) => iter,
                    Self::MapHashSetIter(iter) => iter,
                } {
                    fn len(&self) -> usize;
                }
            }
        }
        match self {
            Self::Mutating { .. } => RegistryIndexTaggedIter::Empty(empty()),
            Self::Frozen {
                values,
                tags,
                #[cfg(debug_assertions)]
                generation,
                ..
            } => {
                #[cfg(debug_assertions)]
                if tag_index.generation != *generation {
                    panic!(
                        "Tag index generation mismatch (Expected {}, found {})",
                        generation, tag_index.generation
                    );
                }
                RegistryIndexTaggedIter::MapHashSetIter(
                    tags[tag_index.index]
                        .iter()
                        .map(|value_index| &values[*value_index]),
                )
            }
        }
    }
    fn contains(&self, key: &NamespacedKey) -> bool {
        match self {
            Self::Mutating { values, .. } => values.contains_key(key),
            Self::Frozen { indices, .. } => indices.contains_key(key),
        }
    }
    fn contains_tag(&self, tag: &NamespacedKey) -> bool {
        match self {
            Self::Mutating { tags, .. } => tags.contains_key(tag),
            Self::Frozen { tag_indices, .. } => tag_indices.contains_key(tag),
        }
    }
    fn is_tagged(&self, tag: &NamespacedKey, key: &NamespacedKey) -> bool {
        match self {
            Self::Mutating { .. } => false,
            Self::Frozen {
                indices,
                tag_indices,
                tags,
                ..
            } => tag_indices
                .get(tag)
                .map(|tag_idx| &tags[*tag_idx])
                .zip(indices.get(key))
                .map_or(false, |(value_indices, value_idx)| {
                    value_indices.contains(value_idx)
                }),
        }
    }
    fn get(&self, key: &NamespacedKey) -> Option<&T> {
        match self {
            Self::Mutating { values, .. } => values.get(key),
            Self::Frozen {
                indices, values, ..
            } => values.get(*indices.get(key)?),
        }
    }
    fn get_tagged(&self, tag: &NamespacedKey) -> impl Iterator<Item = &T> + ExactSizeIterator {
        enum RegistryTaggedIter<'item, T: 'item, L: FnMut(&'item usize) -> &'item T> {
            Empty(Empty<&'item T>),
            MapHashSetIter(Map<std::collections::hash_set::Iter<'item, usize>, L>),
        }
        impl<'item, T: 'item, L: FnMut(&'item usize) -> &'item T> Iterator
            for RegistryTaggedIter<'item, T, L>
        {
            type Item = &'item T;
            delegate! {
                to match self {
                    Self::Empty(iter) => iter,
                    Self::MapHashSetIter(iter) => iter,
                } {
                    fn next(&mut self) -> Option<Self::Item>;
                    fn size_hint(&self) -> (usize, Option<usize>);
                    fn count(self) -> usize;
                    fn fold<B, F>(self, init: B, f: F) -> B where Self: Sized, F: FnMut(B, Self::Item) -> B;
                }
            }
        }
        impl<'item, T: 'item, L: FnMut(&'item usize) -> &'item T> ExactSizeIterator
            for RegistryTaggedIter<'item, T, L>
        {
            delegate! {
                to match self {
                    Self::Empty(iter) => iter,
                    Self::MapHashSetIter(iter) => iter,
                } {
                    fn len(&self) -> usize;
                }
            }
        }
        match self {
            Self::Mutating { .. } => RegistryTaggedIter::Empty(empty()),
            Self::Frozen {
                values,
                tag_indices,
                tags,
                ..
            } => tag_indices
                .get(tag)
                .map(|tag_idx| {
                    RegistryTaggedIter::MapHashSetIter(
                        tags[*tag_idx].iter().map(|value_idx| &values[*value_idx]),
                    )
                })
                .unwrap_or_else(|| RegistryTaggedIter::Empty(empty())),
        }
    }
    fn values(&'_ self) -> impl Iterator<Item = &T> + ExactSizeIterator {
        enum RegistryValuesIter<'item, T> {
            HashMapValues(std::collections::hash_map::Values<'item, NamespacedKey, T>),
            SliceIter(std::slice::Iter<'item, T>),
        }
        impl<'item, T> Iterator for RegistryValuesIter<'item, T> {
            type Item = &'item T;
            delegate! {
                to match self {
                    Self::HashMapValues(iter) => iter,
                    Self::SliceIter(iter) => iter,
                } {
                    fn next(&mut self) -> Option<Self::Item>;
                    fn size_hint(&self) -> (usize, Option<usize>);
                    fn count(self) -> usize;
                    fn fold<B, F>(self, init: B, f: F) -> B where Self: Sized, F: FnMut(B, Self::Item) -> B;
                }
            }
        }
        impl<T> ExactSizeIterator for RegistryValuesIter<'_, T> {
            delegate! {
                to match self {
                    Self::HashMapValues(iter) => iter,
                    Self::SliceIter(iter) => iter,
                } {
                    fn len(&self) -> usize;
                }
            }
        }
        match self {
            Self::Mutating { values, .. } => RegistryValuesIter::HashMapValues(values.values()),
            Self::Frozen { values, .. } => RegistryValuesIter::SliceIter(values.iter()),
        }
    }
    fn register(&mut self, key: NamespacedKey, value: T) -> Result<NamespacedKey, RegistryError> {
        match self {
            Self::Mutating { values, .. } => {
                if values.contains_key(&key) {
                    return Err(RegistryError::ValueAlreadyExists { key });
                }
                values.insert(key.clone(), value);
                Ok(key)
            }
            Self::Frozen { .. } => Err(RegistryError::RegistryFrozen {}),
        }
    }
    #[inline]
    fn register_keyed(&mut self, value: T) -> Result<NamespacedKey, RegistryError>
    where
        T: Keyed,
    {
        self.register(value.key().clone(), value)
    }
    fn register_tag(
        &mut self,
        tag: NamespacedKey,
        items: impl IntoIterator<Item = RegistryTagItem>,
    ) -> Result<NamespacedKey, RegistryError> {
        match self {
            Self::Mutating { tags, .. } => {
                if tags.contains_key(&tag) {
                    return Err(RegistryError::TagAlreadyExists { tag });
                }
                tags.insert(tag.clone(), items.into_iter().collect());
                Ok(tag)
            }
            Self::Frozen { .. } => Err(RegistryError::RegistryFrozen {}),
        }
    }
    fn freeze(&mut self) {
        match self {
            Self::Mutating { values, tags, .. } => {
                let mut indices = HashMap::with_capacity(values.len());
                let mut frozen_values = Vec::with_capacity(values.len());
                for (value_key, value) in take(values) {
                    indices.insert(value_key, frozen_values.len());
                    frozen_values.push(value);
                }
                let mut tag_indices = HashMap::with_capacity(tags.len());
                let mut frozen_tags = Vec::with_capacity(tags.len());
                let mut visited = HashSet::new();
                let mut current_path = Vec::new();
                for (tag_key, tag_items) in tags.iter() {
                    tag_indices.insert(tag_key.clone(), frozen_tags.len());
                    fn resolve_tag_items(
                        tags: &HashMap<NamespacedKey, HashSet<RegistryTagItem>>,
                        value_indices: &HashMap<NamespacedKey, usize>,
                        tag_items: &HashSet<RegistryTagItem>,
                        result: &mut HashSet<usize>,
                        visited_tags: &mut HashSet<NamespacedKey>,
                        current_path: &mut Vec<NamespacedKey>,
                        current_tag: &NamespacedKey,
                    ) {
                        if current_path.contains(current_tag) {
                            panic!(
                                "Cyclic reference in tags: {}",
                                current_path
                                    .iter()
                                    .chain(once(current_tag))
                                    .map(NamespacedKey::to_string)
                                    .collect::<Box<[_]>>()
                                    .join(" -> ")
                            );
                        }
                        current_path.push(current_tag.clone());
                        for item in tag_items {
                            match item {
                                RegistryTagItem::Value(value_key) => {
                                    if let Some(value_index) = value_indices.get(value_key) {
                                        result.insert(*value_index);
                                    } else {
                                        warn!(
                                            "Unknown value key '{}' in tag '{}'",
                                            value_key, current_tag
                                        );
                                    }
                                }
                                RegistryTagItem::Tag(tag_key) => {
                                    if !visited_tags.contains(tag_key) {
                                        visited_tags.insert(tag_key.clone());
                                        if let Some(sub_items) = tags.get(tag_key) {
                                            resolve_tag_items(
                                                tags,
                                                value_indices,
                                                sub_items,
                                                result,
                                                visited_tags,
                                                current_path,
                                                tag_key,
                                            );
                                        } else {
                                            warn!(
                                                "Unknown tag key '{}' in tag '{}'",
                                                tag_key, current_tag
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        current_path.pop();
                    }
                    let mut value_indices = HashSet::new();
                    resolve_tag_items(
                        &tags,
                        &indices,
                        tag_items,
                        &mut value_indices,
                        &mut visited,
                        &mut current_path,
                        tag_key,
                    );
                    frozen_tags.push(value_indices);
                }
                *self = Self::Frozen {
                    indices,
                    values: frozen_values.into_boxed_slice(),
                    tag_indices,
                    tags: frozen_tags.into_boxed_slice(),
                    #[cfg(debug_assertions)]
                    generation: 0,
                };
            }
            Self::Frozen { .. } => panic!("Cannot freeze a frozen registry"),
        }
    }
}

pub type Reg<'world, T> = Res<'world, Registry<T>>;

pub type RegMut<'world, T> = ResMut<'world, Registry<T>>;

/// The `Registry` is a Bevy [resource](Resource) that provides storage and organization of values.
/// It supports [hierarchical tagging](Self::register_tag) and efficient [indexing](Self::get_index).
///
/// # Creation
/// Use [`(Sub)App::init_registry`](RegistryInitExt::init_registry) to create a registry for a type.
///
/// Use [`Registry::clear`](Self::clear) to reset a registry.
///
/// # Note
/// The registry automatically [freezes](Self::is_frozen) one Bevy update cycle after it is created or reset.
/// You can subscribe to [creation events](RegistryEvent::Creation) and [freezing events](RegistryEvent::Freezing) for these state changes.
#[derive(Resource)]
pub struct Registry<T: Send + Sync + 'static> {
    inner: RegistryInner<T>,
    events: Vec<RegistryEvent<T>>,
}

impl<T: Send + Sync + 'static> Registry<T> {
    fn new() -> Self {
        Self {
            inner: RegistryInner::new(
                #[cfg(debug_assertions)]
                0,
            ),
            events: vec![RegistryEvent::new_creation()],
        }
    }
    /// Completely clears the registry, resetting it to a new mutating state.
    ///
    /// Emits a new [RegistryEvent::Creation].
    pub fn clear(&mut self) {
        self.inner = RegistryInner::new(
            #[cfg(debug_assertions)]
            {
                self.inner.generation() + 1
            },
        );
        self.events.clear();
        self.events.push(RegistryEvent::new_creation());
    }
    delegate! {
        to self.inner {
            /// Returns whether the registry is mutating.
            ///
            /// A mutating registry:
            /// - Accepts new values and tags
            /// - Is not indexed
            /// - Has not resolved tags
            ///
            /// See also: [is_frozen](Self::is_frozen)
            pub fn is_mutating(&self) -> bool;
            /// Returns whether the registry is mutating.
            ///
            /// A mutating registry:
            /// - Does not accept new values or tags
            /// - Is indexed
            /// - Has resolved tags
            ///
            /// See also: [is_mutating](Self::is_mutating)
            pub fn is_frozen(&self) -> bool;
            /// Gets the [ValueIndex] for the given key, if it exists.
            /// Always returns `None` if the registry is not [frozen](Self::is_frozen).
            pub fn get_index(&self, key: &NamespacedKey) -> Option<ValueIndex<T>>;
            /// Gets the [TagIndex] for the given tag, if it exists.
            /// Always returns `None` if the registry is not [frozen](Self::is_frozen).
            pub fn get_tag_index(&self, tag: &NamespacedKey) -> Option<TagIndex<T>>;
            /// Retrieves a value by index.
            /// Always returns `None` if the registry is not [frozen](Self::is_frozen).
            ///
            /// # Panics
            /// When debug assertions are enabled, panics if the index generation doesn't match the registry generation.
            pub fn index(&self, value_index: ValueIndex<T>) -> Option<&T>;
            /// Checks whether a value (by key) is tagged with a given tag, by index.
            /// Always returns `false` if the registry is not [frozen](Self::is_frozen).
            ///
            /// # Panics
            /// When debug assertions are enabled, panics if the index generation doesn't match the registry generation.
            pub fn is_tagged_indexed(&self, tag_index: TagIndex<T>, key: &NamespacedKey) -> bool;
            /// Returns an iterator over all values tagged with the given tag, by index.
            /// Always returns an empty iterator if the registry is not [frozen](Self::is_frozen).
            ///
            /// # Panics
            /// When debug assertions are enabled, panics if the index generation doesn't match the registry generation.
            pub fn index_tagged(&self, tag_index: TagIndex<T>) -> impl Iterator<Item = &T> + ExactSizeIterator;
            /// Checks whether a value with the given key exists.
            pub fn contains(&self, key: &NamespacedKey) -> bool;
            /// Checks whether a tag with the given key exists.
            pub fn contains_tag(&self, tag: &NamespacedKey) -> bool;
            /// Checks whether a tag with the given key exists.
            /// Always returns `false` if the registry is not [frozen](Self::is_frozen).
            pub fn is_tagged(&self, tag: &NamespacedKey, key: &NamespacedKey) -> bool;
            /// Retrieves a value by key.
            pub fn get(&self, key: &NamespacedKey) -> Option<&T>;
            /// Returns an iterator over all values tagged with the given tag, by key.
            /// Always returns an empty iterator if the registry is not [frozen](Self::is_frozen).
            pub fn get_tagged(&self, tag: &NamespacedKey) -> impl Iterator<Item = &T> + ExactSizeIterator;
            /// Returns an iterator over all values in the registry.
            pub fn values(&self) -> impl Iterator<Item = &T> + ExactSizeIterator;
            /// Registers a new value with the given key.
            ///
            /// Fails if a value with the same key already exists, or the registry is [frozen](Self::is_frozen).
            pub fn register(&mut self, key: NamespacedKey, value: T) -> Result<NamespacedKey, RegistryError>;
            /// Registers a new [keyed](Keyed) value.
            ///
            /// Fails if a value with the same key already exists, or the registry is [frozen](Self::is_frozen).
            pub fn register_keyed(&mut self, value: T) -> Result<NamespacedKey, RegistryError> where T: Keyed;
            /// Registers a tag with the given key.
            ///
            /// Fails if a tag with the same key already exists, or the registry is [frozen](Self::is_frozen).
            pub fn register_tag(&mut self, tag: NamespacedKey, items: impl IntoIterator<Item = RegistryTagItem>) -> Result<NamespacedKey, RegistryError>;
        }
    }
}

impl<T: Send + Sync + 'static> Index<ValueIndex<T>> for Registry<T> {
    type Output = T;
    delegate! {
        to self.inner {
            #[unwrap]
            fn index(&self, index: ValueIndex<T>) -> &Self::Output;
        }
    }
}

pub type RegBoxed<'world, T> = Res<'world, RegistryBoxed<T>>;

pub type RegBoxedMut<'world, T> = ResMut<'world, RegistryBoxed<T>>;

pub type RegistryBoxed<T> = Registry<Box<T>>;

fn process_registry_events<T: Send + Sync + 'static>(
    mut registry: ResMut<Registry<T>>,
    mut events: ParamSet<(
        MessageReader<RegistryEvent<T>>,
        MessageWriter<RegistryEvent<T>>,
    )>,
) {
    events
        .p1()
        .write_batch(registry.bypass_change_detection().events.drain(..));
    registry.bypass_change_detection().events.shrink_to_fit();
    let freeze_events = events
        .p0()
        .read()
        .filter_map(|event| match event {
            RegistryEvent::Creation(_) => {
                registry.inner.freeze();
                Some(RegistryEvent::new_freezing())
            }
            _ => None,
        })
        .collect::<Box<[_]>>();
    events.p1().write_batch(freeze_events);
}

pub trait RegistryInitExt {
    /// Initializes a new [`Registry<T>`](Registry) and sets up its event system.
    ///
    /// # Examples
    /// ```
    /// # use bevy::prelude::*;
    /// # use crate::reg::RegistryInitExt;
    /// # struct Item;
    /// let mut app = App::new();
    /// app.init_registry::<Item>();
    /// // The registry is now available
    /// assert!(app.world().contains_resource::<Registry<Item>>());
    /// ```
    fn init_registry<T: Send + Sync + 'static>(&mut self) -> &mut Self;
    /// Initializes a new [`RegistryBoxed<T>`](RegistryBoxed) and sets up its event system.
    ///
    /// # Examples
    /// ```
    /// # use bevy::prelude::*;
    /// # use crate::reg::RegistryInitExt;
    /// # trait Lootable {}
    /// let mut app = App::new();
    /// app.init_boxed_registry::<dyn Lootable>();
    /// // The boxed registry is now available
    /// assert!(app.world().contains_resource::<BoxedRegistry<dyn Lootable>>());
    /// ```
    #[inline]
    fn init_registry_boxed<T: ?Sized + Send + Sync + 'static>(&mut self) -> &mut Self {
        self.init_registry::<Box<T>>()
    }
}

impl RegistryInitExt for SubApp {
    #[inline]
    fn init_registry<T: Send + Sync + 'static>(&mut self) -> &mut Self {
        self.insert_resource(Registry::<T>::new());
        self.add_message::<RegistryEvent<T>>();
        self.add_systems(PostUpdate, process_registry_events::<T>);
        self
    }
}

impl RegistryInitExt for App {
    #[inline]
    fn init_registry<T: Send + Sync + 'static>(&mut self) -> &mut Self {
        self.insert_resource(Registry::<T>::new());
        self.add_message::<RegistryEvent<T>>();
        self.add_systems(PostUpdate, process_registry_events::<T>);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_inner<T: 'static>() -> RegistryInner<T> {
        RegistryInner::new(
            #[cfg(debug_assertions)]
            0,
        )
    }

    #[test]
    fn new_registry_is_mutating() {
        let inner = new_inner::<()>();
        assert!(inner.is_mutating());
        assert!(!inner.is_frozen());
    }

    #[test]
    fn registering_and_retrieving_value() {
        let mut inner = new_inner::<i32>();
        let key = NamespacedKey::new("test", "value1");
        let result = inner.register(key.clone(), 42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), key);
        assert!(inner.contains(&key));
        assert_eq!(inner.get(&key), Some(&42));
    }

    #[test]
    fn registering_duplicate_value() {
        let mut inner = new_inner::<i32>();
        let key = NamespacedKey::new("test", "value1");
        inner.register(key.clone(), 42).unwrap();
        let result = inner.register(key.clone(), 100);
        assert!(matches!(result, Err(RegistryError::ValueAlreadyExists { key: k }) if k == key));
    }

    #[test]
    fn registering_keyed_value() {
        let mut inner = new_inner::<(NamespacedKey, i32)>();
        let key = NamespacedKey::new("test", "value1");
        let result = inner.register_keyed((key.clone(), 42));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), key);
        assert!(inner.contains(&key));
        let retrieved = inner.get(&key).unwrap();
        assert_eq!(retrieved.0, key);
        assert_eq!(retrieved.1, 42);
    }

    #[test]
    fn registering_and_retrieving_tag() {
        let mut inner = new_inner::<i32>();
        let tag = NamespacedKey::new("test", "tag1");
        let value_key = NamespacedKey::new("test", "value1");
        inner.register(value_key.clone(), 42).unwrap();
        let result =
            inner.register_tag(tag.clone(), vec![RegistryTagItem::Value(value_key.clone())]);
        assert!(result.is_ok());
        assert!(inner.contains_tag(&tag));
    }

    #[test]
    fn registering_duplicate_tag() {
        let mut inner = new_inner::<i32>();
        let tag = NamespacedKey::new("test", "tag1");
        let items = Vec::new();
        inner.register_tag(tag.clone(), items).unwrap();
        let result = inner.register_tag(tag.clone(), vec![]);
        assert!(matches!(result, Err(RegistryError::TagAlreadyExists { tag: t }) if t == tag));
    }

    #[test]
    fn freezing() {
        let mut inner = new_inner::<i32>();
        let key = NamespacedKey::new("test", "value1");
        inner.register(key.clone(), 42).unwrap();
        assert!(inner.is_mutating());
        inner.freeze();
        assert!(inner.is_frozen());
        assert!(!inner.is_mutating());
    }

    #[test]
    #[should_panic(expected = "Cannot freeze a frozen registry")]
    fn freezing_frozen() {
        let mut inner = new_inner::<i32>();
        inner.freeze();
        inner.freeze();
    }

    #[test]
    fn registering_after_freezing() {
        let mut inner = new_inner::<i32>();
        inner.freeze();
        let key = NamespacedKey::new("test", "value1");
        let result = inner.register(key.clone(), 42);
        assert!(matches!(result, Err(RegistryError::RegistryFrozen {})));
    }

    #[test]
    fn indexing_after_freezing() {
        let mut inner = new_inner::<i32>();
        let key1 = NamespacedKey::new("test", "value1");
        let key2 = NamespacedKey::new("test", "value2");
        inner.register(key1.clone(), 42).unwrap();
        inner.register(key2.clone(), 100).unwrap();
        inner.freeze();
        let idx1 = inner.get_index(&key1);
        let idx2 = inner.get_index(&key2);
        assert!(idx1.is_some());
        assert!(idx2.is_some());
        assert_ne!(idx1.unwrap().index, idx2.unwrap().index);
    }

    #[test]
    fn indexing_before_freezing() {
        let mut inner = new_inner::<i32>();
        let key = NamespacedKey::new("test", "value1");
        inner.register(key.clone(), 42).unwrap();
        assert!(inner.get_index(&key).is_none());
    }

    #[test]
    fn index_retrieval_after_freezing() {
        let mut inner = new_inner::<i32>();
        let key = NamespacedKey::new("test", "value1");
        inner.register(key.clone(), 42).unwrap();
        inner.freeze();
        let value_idx = inner.get_index(&key).unwrap();
        let retrieved = inner.index(value_idx);
        assert_eq!(retrieved, Some(&42));
    }

    #[test]
    fn index_retrieval_before_freezing() {
        let mut inner = new_inner::<i32>();
        let key = NamespacedKey::new("test", "value1");
        inner.register(key.clone(), 42).unwrap();
        let value_idx = ValueIndex::new(
            0,
            #[cfg(debug_assertions)]
            0,
        );
        assert!(inner.index(value_idx).is_none());
    }

    #[test]
    fn tag_indexing_after_freezing() {
        let mut inner = new_inner::<i32>();
        let tag = NamespacedKey::new("test", "tag1");
        let value_key = NamespacedKey::new("test", "value1");
        inner.register(value_key.clone(), 42).unwrap();
        inner
            .register_tag(tag.clone(), vec![RegistryTagItem::Value(value_key.clone())])
            .unwrap();
        inner.freeze();
        let tag_idx = inner.get_tag_index(&tag).unwrap();
        assert!(inner.is_tagged_indexed(tag_idx, &value_key));
        let tagged_values: Vec<&i32> = inner.index_tagged(tag_idx).collect();
        assert_eq!(tagged_values, vec![&42]);
    }

    #[test]
    fn nested_tag_resolution() {
        let mut inner = new_inner::<i32>();
        let value1 = NamespacedKey::new("test", "value1");
        let value2 = NamespacedKey::new("test", "value2");
        let inner_tag = NamespacedKey::new("test", "inner_tag");
        let outer_tag = NamespacedKey::new("test", "outer_tag");
        inner.register(value1.clone(), 42).unwrap();
        inner.register(value2.clone(), 100).unwrap();
        inner
            .register_tag(
                inner_tag.clone(),
                vec![RegistryTagItem::Value(value1.clone())],
            )
            .unwrap();
        inner
            .register_tag(
                outer_tag.clone(),
                vec![
                    RegistryTagItem::Tag(inner_tag.clone()),
                    RegistryTagItem::Value(value2.clone()),
                ],
            )
            .unwrap();
        inner.freeze();
        let outer_tagged: Vec<&i32> = inner.get_tagged(&outer_tag).collect();
        assert_eq!(outer_tagged.len(), 2);
        assert!(outer_tagged.contains(&&42));
        assert!(outer_tagged.contains(&&100));
        assert!(inner.is_tagged(&outer_tag, &value1));
        assert!(inner.is_tagged(&outer_tag, &value2));
        assert!(inner.is_tagged(&inner_tag, &value1));
        assert!(!inner.is_tagged(&inner_tag, &value2));
    }

    #[test]
    #[should_panic(expected = "Cyclic reference in tags")]
    fn cyclic_tag_reference() {
        let mut inner = new_inner::<i32>();
        let tag1 = NamespacedKey::new("test", "tag1");
        let tag2 = NamespacedKey::new("test", "tag2");
        inner
            .register_tag(tag1.clone(), vec![RegistryTagItem::Tag(tag2.clone())])
            .unwrap();
        inner
            .register_tag(tag2.clone(), vec![RegistryTagItem::Tag(tag1.clone())])
            .unwrap();
        inner.freeze();
    }

    #[test]
    fn values_iterator() {
        let mut inner = new_inner::<i32>();
        let key1 = NamespacedKey::new("test", "value1");
        let key2 = NamespacedKey::new("test", "value2");
        inner.register(key1.clone(), 42).unwrap();
        inner.register(key2.clone(), 100).unwrap();
        let mut values: Vec<&i32> = inner.values().collect();
        values.sort();
        assert_eq!(values, vec![&42, &100]);
        inner.freeze();
        let values: Vec<&i32> = inner.values().collect();
        assert_eq!(values.len(), 2);
        assert!(values.contains(&&42));
        assert!(values.contains(&&100));
    }

    #[test]
    fn getting_tagged_before_freezing() {
        let mut inner = new_inner::<i32>();
        let tag = NamespacedKey::new("test", "tag1");
        let value_key = NamespacedKey::new("test", "value1");
        inner.register(value_key.clone(), 42).unwrap();
        inner
            .register_tag(tag.clone(), vec![RegistryTagItem::Value(value_key.clone())])
            .unwrap();
        let tagged: Vec<&i32> = inner.get_tagged(&tag).collect();
        assert!(tagged.is_empty());
    }

    #[test]
    fn unknown_value_in_tag() {
        let mut inner = new_inner::<i32>();
        let tag = NamespacedKey::new("test", "tag1");
        let unknown_key = NamespacedKey::new("test", "unknown");
        inner
            .register_tag(
                tag.clone(),
                vec![RegistryTagItem::Value(unknown_key.clone())],
            )
            .unwrap();
        inner.freeze();
        let tagged: Vec<&i32> = inner.get_tagged(&tag).collect();
        assert!(tagged.is_empty());
    }

    #[test]
    fn unknown_tag_in_tag() {
        let mut inner = new_inner::<i32>();
        let tag = NamespacedKey::new("test", "tag1");
        let unknown_tag = NamespacedKey::new("test", "unknown_tag");
        inner
            .register_tag(tag.clone(), vec![RegistryTagItem::Tag(unknown_tag.clone())])
            .unwrap();
        inner.freeze();
        let tagged: Vec<&i32> = inner.get_tagged(&tag).collect();
        assert!(tagged.is_empty());
    }

    #[test]
    fn exact_size_iterator_impls() {
        let mut inner = new_inner::<i32>();
        let key1 = NamespacedKey::new("test", "value1");
        let key2 = NamespacedKey::new("test", "value2");
        inner.register(key1, 42).unwrap();
        inner.register(key2, 100).unwrap();
        inner.freeze();
        let values_iter = inner.values();
        assert_eq!(values_iter.len(), 2);
        let count = values_iter.count();
        assert_eq!(count, 2);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "Value index generation mismatch")]
    fn value_index_generation_mismatch() {
        let mut inner = new_inner::<i32>();
        inner.freeze();
        let _ = inner.index(ValueIndex::new(0, 42));
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "Tag index generation mismatch")]
    fn tag_index_generation_mismatch() {
        let mut inner = new_inner::<i32>();
        inner.freeze();
        let _ = inner.is_tagged_indexed(TagIndex::new(0, 42), &NamespacedKey::new("test", "value"));
    }

    #[test]
    fn registry_clear_resetting_state() {
        let mut registry = Registry::<i32>::new();
        let key = NamespacedKey::new("test", "value1");
        registry.register(key.clone(), 42).unwrap();
        registry.inner.freeze();
        assert!(registry.is_frozen());
        registry.clear();
        assert!(registry.is_mutating());
        assert!(!registry.is_frozen());
        assert!(!registry.contains(&key));
    }
}
