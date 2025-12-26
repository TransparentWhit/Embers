use crate::utils::{Keyed, NamespacedKey};
use bevy::prelude::*;
use std::collections::HashMap;
use std::marker::PhantomData;
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
        match &self {
            Ok(..) => self,
            Err(..) => match registry.get(key) {
                Some(value) => Ok(value.clone()),
                None => self,
            },
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
            Some(value) => value,
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
            Ok(value) => value,
            Err(_err) => registry.get(key).cloned().unwrap(),
        }
    }
}

pub trait RegistryAccess {
    type Item: ?Sized;
    fn contains(&self, key: &NamespacedKey) -> bool;
    fn get(&self, key: &NamespacedKey) -> Option<&Self::Item>;
    fn iter(&self) -> impl Iterator<Item = &Self::Item>;
    fn iter_entries(&self) -> impl Iterator<Item = (&NamespacedKey, &Self::Item)>;
}

#[derive(Debug, Message)]
pub enum RegistryEvent<T: ?Sized> {
    EntryRegistration {
        key: NamespacedKey,
        _marker: PhantomData<T>,
    },
}

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("An entry with such a key already exists: {key}")]
    EntryAlreadyExists { key: NamespacedKey },
}

#[derive(Resource)]
pub struct Registry<T: Send + Sync + 'static> {
    entries: HashMap<NamespacedKey, T>,
    events: Vec<RegistryEvent<T>>,
}

impl<T: Send + Sync + 'static> Default for Registry<T> {
    fn default() -> Self {
        Self {
            entries: Default::default(),
            events: Default::default(),
        }
    }
}

impl<T: Send + Sync + 'static> Registry<T> {
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }
    pub fn contains(&self, key: &NamespacedKey) -> bool {
        self.entries.contains_key(key)
    }
    pub fn get(&self, key: &NamespacedKey) -> Option<&T> {
        self.entries.get(key)
    }
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.entries.values()
    }
    pub fn iter_entries(&self) -> impl Iterator<Item = (&NamespacedKey, &T)> {
        self.entries.iter()
    }
    pub fn register(&mut self, key: NamespacedKey, value: T) -> Result<(), RegistryError> {
        if self.entries.contains_key(&key) {
            return Err(RegistryError::EntryAlreadyExists { key });
        }
        self.entries.insert(key.clone(), value);
        self.events.push(RegistryEvent::EntryRegistration {
            key,
            _marker: PhantomData,
        });
        Ok(())
    }
}

impl<T: Keyed + Send + Sync + 'static> Registry<T> {
    #[inline]
    pub fn register_keyed(&mut self, value: T) -> Result<(), RegistryError> {
        self.register(value.key().clone(), value)
    }
}

impl<T: Send + Sync + 'static> RegistryAccess for &Registry<T> {
    type Item = T;
    #[inline]
    fn contains(&self, key: &NamespacedKey) -> bool {
        (*self).contains(key)
    }
    #[inline]
    fn get(&self, key: &NamespacedKey) -> Option<&Self::Item> {
        (*self).get(key)
    }
    #[inline]
    fn iter(&self) -> impl Iterator<Item = &Self::Item> {
        (*self).iter()
    }
    #[inline]
    fn iter_entries(&self) -> impl Iterator<Item = (&NamespacedKey, &Self::Item)> {
        (*self).iter_entries()
    }
}

#[derive(Resource)]
pub struct DynamicRegistry<T: ?Sized + Send + Sync + 'static> {
    entries: HashMap<NamespacedKey, Box<T>>,
    events: Vec<RegistryEvent<T>>,
}

impl<T: ?Sized + Send + Sync + 'static> Default for DynamicRegistry<T> {
    fn default() -> Self {
        Self {
            entries: Default::default(),
            events: Default::default(),
        }
    }
}

impl<T: ?Sized + Send + Sync + 'static> DynamicRegistry<T> {
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }
    pub fn contains(&self, key: &NamespacedKey) -> bool {
        self.entries.contains_key(key)
    }
    pub fn get(&self, key: &NamespacedKey) -> Option<&T> {
        self.entries
            .get(key)
            .map(|boxed_value| boxed_value.as_ref())
    }
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.entries
            .values()
            .map(|boxed_value| boxed_value.as_ref())
    }
    pub fn iter_entries(&self) -> impl Iterator<Item = (&NamespacedKey, &T)> {
        self.entries
            .iter()
            .map(|(key, boxed_value)| (key, boxed_value.as_ref()))
    }
    #[inline]
    pub fn register(&mut self, key: NamespacedKey, value: T) -> Result<(), RegistryError>
    where
        T: Sized,
    {
        self.register_boxed(key, Box::new(value))
    }
    pub fn register_boxed(
        &mut self,
        key: NamespacedKey,
        boxed_value: Box<T>,
    ) -> Result<(), RegistryError> {
        if self.entries.contains_key(&key) {
            return Err(RegistryError::EntryAlreadyExists { key });
        }
        self.entries.insert(key.clone(), boxed_value);
        self.events.push(RegistryEvent::EntryRegistration {
            key,
            _marker: PhantomData,
        });
        Ok(())
    }
}

impl<T: ?Sized + Keyed + Send + Sync + 'static> DynamicRegistry<T> {
    #[inline]
    pub fn register_keyed(&mut self, value: T) -> Result<(), RegistryError>
    where
        T: Sized,
    {
        self.register(value.key().clone(), value)
    }
    #[inline]
    pub fn register_boxed_keyed(&mut self, boxed_value: Box<T>) -> Result<(), RegistryError> {
        self.register_boxed(boxed_value.key().clone(), boxed_value)
    }
}

impl<T: ?Sized + Send + Sync + 'static> RegistryAccess for &DynamicRegistry<T> {
    type Item = T;
    #[inline]
    fn contains(&self, key: &NamespacedKey) -> bool {
        (*self).contains(key)
    }
    #[inline]
    fn get(&self, key: &NamespacedKey) -> Option<&Self::Item> {
        (*self).get(key)
    }
    #[inline]
    fn iter(&self) -> impl Iterator<Item = &Self::Item> {
        (*self).iter()
    }
    #[inline]
    fn iter_entries(&self) -> impl Iterator<Item = (&NamespacedKey, &Self::Item)> {
        (*self).iter_entries()
    }
}

fn broadcast_registry_events<T: Send + Sync + 'static>(
    mut registry: ResMut<Registry<T>>,
    mut events_writer: MessageWriter<RegistryEvent<T>>,
) {
    for event in registry.bypass_change_detection().events.drain(..) {
        events_writer.write(event);
    }
    registry.bypass_change_detection().events.shrink_to_fit();
}

fn broadcast_dynamic_registry_events<T: ?Sized + Send + Sync + 'static>(
    mut registry: ResMut<DynamicRegistry<T>>,
    mut events_writer: MessageWriter<RegistryEvent<T>>,
) {
    for event in registry.bypass_change_detection().events.drain(..) {
        events_writer.write(event);
    }
    registry.bypass_change_detection().events.shrink_to_fit();
}

pub trait RegistryInitExt {
    fn init_registry<T: Send + Sync + 'static>(&mut self) -> &mut Self;
    fn init_dynamic_registry<T: ?Sized + Send + Sync + 'static>(&mut self) -> &mut Self;
}

impl RegistryInitExt for SubApp {
    #[inline]
    fn init_registry<T: Send + Sync + 'static>(&mut self) -> &mut Self {
        self.init_resource::<Registry<T>>();
        self.add_message::<RegistryEvent<T>>();
        self.add_systems(PostUpdate, broadcast_registry_events::<T>);
        self
    }
    #[inline]
    fn init_dynamic_registry<T: ?Sized + Send + Sync + 'static>(&mut self) -> &mut Self {
        self.init_resource::<DynamicRegistry<T>>();
        self.add_message::<RegistryEvent<T>>();
        self.add_systems(PostUpdate, broadcast_dynamic_registry_events::<T>);
        self
    }
}

impl RegistryInitExt for App {
    #[inline]
    fn init_registry<T: Send + Sync + 'static>(&mut self) -> &mut Self {
        self.init_resource::<Registry<T>>();
        self.add_message::<RegistryEvent<T>>();
        self.add_systems(PostUpdate, broadcast_registry_events::<T>);
        self
    }
    #[inline]
    fn init_dynamic_registry<T: ?Sized + Send + Sync + 'static>(&mut self) -> &mut Self {
        self.init_resource::<DynamicRegistry<T>>();
        self.add_message::<RegistryEvent<T>>();
        self.add_systems(PostUpdate, broadcast_dynamic_registry_events::<T>);
        self
    }
}
