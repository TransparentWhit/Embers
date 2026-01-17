use crate::utils::{Keyed, NamespacedKey};
use bevy::prelude::*;
use delegate::delegate;
use std::collections::{HashMap, HashSet};
use std::iter::{Empty, Map, empty, once};
use std::marker::PhantomData;
use std::mem::take;
use std::ops::{Deref, Index};
use thiserror::Error;

pub trait OrRegistry {
    type Item;
    fn or_registry(
        self,
        registry: impl RegistryAccess<Item = Self::Item>,
        key: &NamespacedKey,
    ) -> Self;
}

impl<T: Clone> OrRegistry for Option<T> {
    type Item = T;
    #[inline]
    fn or_registry(
        self,
        registry: impl RegistryAccess<Item = Self::Item>,
        key: &NamespacedKey,
    ) -> Self {
        match &self {
            Some(..) => self,
            None => registry.get(key).cloned(),
        }
    }
}

impl<T: Clone, E> OrRegistry for Result<T, E> {
    type Item = T;
    #[inline]
    fn or_registry(
        self,
        registry: impl RegistryAccess<Item = Self::Item>,
        key: &NamespacedKey,
    ) -> Self {
        match self {
            Ok(val) => Ok(val),
            Err(err) => registry.get(key).cloned().ok_or(err),
        }
    }
}

pub trait UnwrapOrRegistry {
    type Item;
    fn unwrap_or_registry(
        self,
        registry: impl RegistryAccess<Item = Self::Item>,
        key: &NamespacedKey,
    ) -> Self::Item;
}

impl<T: Clone> UnwrapOrRegistry for Option<T> {
    type Item = T;
    #[inline]
    fn unwrap_or_registry(
        self,
        registry: impl RegistryAccess<Item = Self::Item>,
        key: &NamespacedKey,
    ) -> Self::Item {
        match self {
            Some(val) => val,
            None => registry.get(key).cloned().unwrap(),
        }
    }
}

impl<T: Clone, E> UnwrapOrRegistry for Result<T, E> {
    type Item = T;
    #[inline]
    fn unwrap_or_registry(
        self,
        registry: impl RegistryAccess<Item = Self::Item>,
        key: &NamespacedKey,
    ) -> Self::Item {
        match self {
            Ok(val) => val,
            Err(_err) => registry.get(key).cloned().unwrap(),
        }
    }
}

pub trait RegistryAccess: Index<ValueIndex<Self::Item>, Output = Self::Item> {
    type Item: ?Sized;
    fn is_mutating(&self) -> bool;
    fn is_frozen(&self) -> bool;
    fn get_index(&self, key: &NamespacedKey) -> Option<ValueIndex<Self::Item>>;
    fn get_tag_index(&self, tag: &NamespacedKey) -> Option<TagIndex<Self::Item>>;
    fn index(&self, value_index: ValueIndex<Self::Item>) -> Option<&Self::Item>;
    fn is_tagged_indexed(&self, tag_index: TagIndex<Self::Item>, key: &NamespacedKey) -> bool;
    fn index_tagged(
        &self,
        tag_index: TagIndex<Self::Item>,
    ) -> impl Iterator<Item = &Self::Item> + ExactSizeIterator;
    fn contains(&self, key: &NamespacedKey) -> bool;
    fn contains_tag(&self, tag: &NamespacedKey) -> bool;
    fn is_tagged(&self, tag: &NamespacedKey, key: &NamespacedKey) -> bool;
    fn get(&self, key: &NamespacedKey) -> Option<&Self::Item>;
    fn get_tagged(
        &self,
        tag: &NamespacedKey,
    ) -> impl Iterator<Item = &Self::Item> + ExactSizeIterator;
    fn values(&self) -> impl Iterator<Item = &Self::Item> + ExactSizeIterator;
}

pub struct RegistryCreateEvent<T: ?Sized> {
    _marker: PhantomData<T>,
}

pub struct RegistryFreezeEvent<T: ?Sized> {
    _marker: PhantomData<T>,
}

#[derive(Message)]
enum RegistryEvent<T: ?Sized> {
    Creation(RegistryCreateEvent<T>),
    Freezing(RegistryFreezeEvent<T>),
}

impl<T: ?Sized> RegistryEvent<T> {
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

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("A value with such a key already exists: {key}")]
    ValueAlreadyExists { key: NamespacedKey },
    #[error("A tag with such a key already exists: {tag}")]
    TagAlreadyExists { tag: NamespacedKey },
    #[error("This operation cannot be performed on a frozen registry")]
    RegistryFrozen {},
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum RegistryTagItem {
    Value(NamespacedKey),
    Tag(NamespacedKey),
}

#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub struct ValueIndex<M: ?Sized> {
    index: usize,
    #[cfg(debug_assertions)]
    generation: u32,
    _marker: PhantomData<M>,
}

impl<M: ?Sized> ValueIndex<M> {
    fn new(index: usize, #[cfg(debug_assertions)] generation: u32) -> Self {
        Self {
            index,
            #[cfg(debug_assertions)]
            generation,
            _marker: PhantomData,
        }
    }
}

#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub struct TagIndex<M: ?Sized> {
    index: usize,
    #[cfg(debug_assertions)]
    generation: u32,
    _marker: PhantomData<M>,
}

impl<M: ?Sized> TagIndex<M> {
    fn new(index: usize, #[cfg(debug_assertions)] generation: u32) -> Self {
        Self {
            index,
            #[cfg(debug_assertions)]
            generation,
            _marker: PhantomData,
        }
    }
}

enum RegistryInner<T: 'static, M: ?Sized> {
    Mutating {
        values: HashMap<NamespacedKey, T>,
        tags: HashMap<NamespacedKey, HashSet<RegistryTagItem>>,
        #[cfg(debug_assertions)]
        generation: u32,
        _marker: PhantomData<M>,
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

impl<T: 'static, M: ?Sized> RegistryInner<T, M> {
    fn new(#[cfg(debug_assertions)] generation: u32) -> Self {
        Self::Mutating {
            values: HashMap::new(),
            tags: HashMap::new(),
            #[cfg(debug_assertions)]
            generation,
            _marker: PhantomData,
        }
    }
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
    fn get_index(&self, key: &NamespacedKey) -> Option<ValueIndex<M>> {
        match self {
            Self::Mutating { .. } => None,
            Self::Frozen {
                indices,
                generation,
                ..
            } => Some(ValueIndex::new(
                *indices.get(key)?,
                #[cfg(debug_assertions)]
                *generation,
            )),
        }
    }
    fn get_tag_index(&self, tag: &NamespacedKey) -> Option<TagIndex<M>> {
        match self {
            Self::Mutating { .. } => None,
            Self::Frozen {
                tag_indices,
                generation,
                ..
            } => Some(TagIndex::new(
                *tag_indices.get(tag)?,
                #[cfg(debug_assertions)]
                *generation,
            )),
        }
    }
    fn index(&self, value_index: ValueIndex<M>) -> Option<&T> {
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
    fn is_tagged_indexed(&self, tag_index: TagIndex<M>, key: &NamespacedKey) -> bool {
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
    fn index_tagged(&self, tag_index: TagIndex<M>) -> impl Iterator<Item = &T> + ExactSizeIterator {
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
                .map(|tag_index| &tags[*tag_index])
                .zip(indices.get(key))
                .map_or(false, |(value_indices, value_index)| {
                    value_indices.contains(value_index)
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
                .map(|tag_index| {
                    RegistryTaggedIter::MapHashSetIter(
                        tags[*tag_index]
                            .iter()
                            .map(|value_index| &values[*value_index]),
                    )
                })
                .unwrap_or_else(|| RegistryTaggedIter::Empty(empty())),
        }
    }
    fn values(&self) -> impl Iterator<Item = &T> + ExactSizeIterator {
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
        items: impl Iterator<Item = RegistryTagItem>,
    ) -> Result<NamespacedKey, RegistryError> {
        match self {
            Self::Mutating { tags, .. } => {
                if tags.contains_key(&tag) {
                    return Err(RegistryError::TagAlreadyExists { tag });
                }
                tags.insert(tag.clone(), items.collect());
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

#[derive(Resource)]
pub struct Registry<T: Send + Sync + 'static> {
    inner: RegistryInner<T, T>,
    events: Vec<RegistryEvent<T>>,
}

impl<T: Send + Sync + 'static> Registry<T> {
    fn new() -> Self {
        Self {
            inner: RegistryInner::new(0),
            events: vec![RegistryEvent::new_creation()],
        }
    }
    pub fn clear(&mut self) {
        self.inner = RegistryInner::new(self.inner.generation() + 1);
        self.events.clear();
        self.events.push(RegistryEvent::new_creation());
    }
    delegate! {
        to self.inner {
            pub fn is_mutating(&self) -> bool;
            pub fn is_frozen(&self) -> bool;
            pub fn get_index(&self, key: &NamespacedKey) -> Option<ValueIndex<T>>;
            pub fn get_tag_index(&self, tag: &NamespacedKey) -> Option<TagIndex<T>>;
            pub fn index(&self, key: ValueIndex<T>) -> Option<&T>;
            pub fn is_tagged_indexed(&self, tag_index: TagIndex<T>, key: &NamespacedKey) -> bool;
            pub fn index_tagged(&self, key: TagIndex<T>) -> impl Iterator<Item = &T> + ExactSizeIterator;
            pub fn contains(&self, key: &NamespacedKey) -> bool;
            pub fn contains_tag(&self, tag: &NamespacedKey) -> bool;
            pub fn is_tagged(&self, tag: &NamespacedKey, key: &NamespacedKey) -> bool;
            pub fn get(&self, key: &NamespacedKey) -> Option<&T>;
            pub fn get_tagged(&self, tag: &NamespacedKey) -> impl Iterator<Item = &T> + ExactSizeIterator;
            pub fn values(&self) -> impl Iterator<Item = &T> + ExactSizeIterator;
            pub fn register(&mut self, key: NamespacedKey, value: T) -> Result<NamespacedKey, RegistryError>;
            pub fn register_keyed(&mut self, value: T) -> Result<NamespacedKey, RegistryError> where T: Keyed;
            pub fn register_tag(&mut self, tag: NamespacedKey, items: impl Iterator<Item = RegistryTagItem>) -> Result<NamespacedKey, RegistryError>;
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

macro_rules! impl_registry_access {
    ($type_: ty, $get_registry: ident) => {
        impl<T: Send + Sync + 'static> Index<ValueIndex<T>> for $type_ {
            type Output = T;
            delegate! {
                to self.$get_registry() {
                    #[unwrap]
                    fn index(&self, value_index: ValueIndex<T>) -> &Self::Output;
                }
            }
        }
        impl<T: Send + Sync + 'static> RegistryAccess for $type_ {
            type Item = T;
            delegate! {
                to self.$get_registry() {
                    fn is_mutating(&self) -> bool;
                    fn is_frozen(&self) -> bool;
                    fn get_index(&self, key: &NamespacedKey) -> Option<ValueIndex<T>>;
                    fn get_tag_index(&self, tag: &NamespacedKey) -> Option<TagIndex<T>>;
                    fn index(&self, value_index: ValueIndex<T>) -> Option<&T>;
                    fn is_tagged_indexed(&self, tag_index: TagIndex<T>, key: &NamespacedKey) -> bool;
                    fn index_tagged(&self, tag_index: TagIndex<T>) -> impl Iterator<Item = &T> + ExactSizeIterator;
                    fn contains(&self, key: &NamespacedKey) -> bool;
                    fn contains_tag(&self, tag: &NamespacedKey) -> bool;
                    fn is_tagged(&self, tag: &NamespacedKey, key: &NamespacedKey) -> bool;
                    fn get(&self, key: &NamespacedKey) -> Option<&T>;
                    fn get_tagged(&self, tag: &NamespacedKey) -> impl Iterator<Item = &T> + ExactSizeIterator;
                    fn values(&self) -> impl Iterator<Item = &T> + ExactSizeIterator;
                }
            }
        }
    };
}

impl_registry_access!(&Reg<'_, T>, as_ref);
impl_registry_access!(&RegMut<'_, T>, as_ref);
impl_registry_access!(&Registry<T>, deref);

pub type DynReg<'world, T> = Res<'world, DynamicRegistry<T>>;

pub type DynRegMut<'world, T> = ResMut<'world, DynamicRegistry<T>>;

#[derive(Resource)]
pub struct DynamicRegistry<T: ?Sized + Send + Sync + 'static> {
    inner: RegistryInner<Box<T>, T>,
    events: Vec<RegistryEvent<T>>,
}

impl<T: ?Sized + Send + Sync + 'static> DynamicRegistry<T> {
    fn new() -> Self {
        Self {
            inner: RegistryInner::new(0),
            events: Vec::new(),
        }
    }
    pub fn clear(&mut self) {
        self.inner = RegistryInner::new(self.inner.generation() + 1);
        self.events.clear();
    }
    delegate! {
        to self.inner {
            pub fn is_mutating(&self) -> bool;
            pub fn is_frozen(&self) -> bool;
            pub fn get_index(&self, key: &NamespacedKey) -> Option<ValueIndex<T>>;
            pub fn get_tag_index(&self, tag: &NamespacedKey) -> Option<TagIndex<T>>;
            #[expr($.map(|boxed_value| boxed_value.as_ref()))]
            pub fn index(&self, key: ValueIndex<T>) -> Option<&T>;
            pub fn is_tagged_indexed(&self, tag_index: TagIndex<T>, key: &NamespacedKey) -> bool;
            #[expr($.map(|boxed_value| boxed_value.as_ref()))]
            pub fn index_tagged(&self, key: TagIndex<T>) -> impl Iterator<Item = &T> + ExactSizeIterator;
            pub fn contains(&self, key: &NamespacedKey) -> bool;
            pub fn contains_tag(&self, tag: &NamespacedKey) -> bool;
            pub fn is_tagged(&self, tag: &NamespacedKey, key: &NamespacedKey) -> bool;
            #[expr($.map(|boxed_value| boxed_value.as_ref()))]
            pub fn get(&self, key: &NamespacedKey) -> Option<&T>;
            #[expr($.map(|boxed_value| boxed_value.as_ref()))]
            pub fn get_tagged(&self, tag: &NamespacedKey) -> impl Iterator<Item = &T> + ExactSizeIterator;
            #[expr($.map(|boxed_value| boxed_value.as_ref()))]
            pub fn values(&self) -> impl Iterator<Item = &T> + ExactSizeIterator;
            #[call(register)]
            pub fn register_boxed(&mut self, key: NamespacedKey, boxed_value: Box<T>) -> Result<NamespacedKey, RegistryError>;
            #[call(register_keyed)]
            pub fn register_boxed_keyed(&mut self, boxed_value: Box<T>) -> Result<NamespacedKey, RegistryError> where T: Keyed;
            pub fn register_tag(&mut self, tag: NamespacedKey, items: impl Iterator<Item = RegistryTagItem>) -> Result<NamespacedKey, RegistryError>;
        }
    }
    #[inline]
    pub fn register(&mut self, key: NamespacedKey, value: T) -> Result<NamespacedKey, RegistryError>
    where
        T: Sized,
    {
        self.register_boxed(key, Box::new(value))
    }
    #[inline]
    pub fn register_keyed(&mut self, value: T) -> Result<NamespacedKey, RegistryError>
    where
        T: Keyed + Sized,
    {
        self.register(value.key().clone(), value)
    }
}

impl<T: ?Sized + Send + Sync + 'static> Index<ValueIndex<T>> for DynamicRegistry<T> {
    type Output = T;
    delegate! {
        to self.inner {
            #[unwrap]
            fn index(&self, index: ValueIndex<T>) -> &Self::Output;
        }
    }
}

macro_rules! impl_dyn_registry_access {
    ($type_: ty, $get_registry: ident) => {
        impl<T: ?Sized + Send + Sync + 'static> Index<ValueIndex<T>> for $type_ {
            type Output = T;
            delegate! {
                to self.$get_registry() {
                    #[unwrap]
                    fn index(&self, key: ValueIndex<T>) -> &Self::Output;
                }
            }
        }
        impl<T: ?Sized + Send + Sync + 'static> RegistryAccess for $type_ {
            type Item = T;
            delegate! {
                to self.$get_registry() {
                    fn is_mutating(&self) -> bool;
                    fn is_frozen(&self) -> bool;
                    fn get_index(&self, key: &NamespacedKey) -> Option<ValueIndex<T>>;
                    fn get_tag_index(&self, tag: &NamespacedKey) -> Option<TagIndex<T>>;
                    fn index(&self, value_index: ValueIndex<T>) -> Option<&T>;
                    fn is_tagged_indexed(&self, tag_index: TagIndex<T>, key: &NamespacedKey) -> bool;
                    fn index_tagged(&self, tag_index: TagIndex<T>) -> impl Iterator<Item = &T> + ExactSizeIterator;
                    fn contains(&self, key: &NamespacedKey) -> bool;
                    fn contains_tag(&self, tag: &NamespacedKey) -> bool;
                    fn is_tagged(&self, tag: &NamespacedKey, key: &NamespacedKey) -> bool;
                    fn get(&self, key: &NamespacedKey) -> Option<&T>;
                    fn get_tagged(&self, tag: &NamespacedKey) -> impl Iterator<Item = &T> + ExactSizeIterator;
                    fn values(&self) -> impl Iterator<Item = &T> + ExactSizeIterator;
                }
            }
        }
    };
}

impl_dyn_registry_access!(&DynReg<'_, T>, as_ref);
impl_dyn_registry_access!(&DynRegMut<'_, T>, as_ref);
impl_dyn_registry_access!(&DynamicRegistry<T>, deref);

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

fn process_dynamic_registry_events<T: ?Sized + Send + Sync + 'static>(
    mut registry: ResMut<DynamicRegistry<T>>,
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
    fn init_registry<T: Send + Sync + 'static>(&mut self) -> &mut Self;
    fn init_dynamic_registry<T: ?Sized + Send + Sync + 'static>(&mut self) -> &mut Self;
}

impl RegistryInitExt for SubApp {
    #[inline]
    fn init_registry<T: Send + Sync + 'static>(&mut self) -> &mut Self {
        self.insert_resource(Registry::<T>::new());
        self.add_message::<RegistryEvent<T>>();
        self.add_systems(PostUpdate, process_registry_events::<T>);
        self
    }
    #[inline]
    fn init_dynamic_registry<T: ?Sized + Send + Sync + 'static>(&mut self) -> &mut Self {
        self.insert_resource(DynamicRegistry::<T>::new());
        self.add_message::<RegistryEvent<T>>();
        self.add_systems(PostUpdate, process_dynamic_registry_events::<T>);
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
    #[inline]
    fn init_dynamic_registry<T: ?Sized + Send + Sync + 'static>(&mut self) -> &mut Self {
        self.insert_resource(DynamicRegistry::<T>::new());
        self.add_message::<RegistryEvent<T>>();
        self.add_systems(PostUpdate, process_dynamic_registry_events::<T>);
        self
    }
}
