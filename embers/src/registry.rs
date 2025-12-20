use crate::utils::NamespacedKey;
use bevy::prelude::*;
use std::any::Any;
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

impl<T> OrRegistry for Option<T> {
    type Item = T;
    #[inline]
    fn or_registry(
        self,
        registry: impl RegistryAccess<Item = Self::Item>,
        key: &NamespacedKey,
    ) -> Self {
        match &self {
            Some(..) => self,
            None => registry.get(key),
        }
    }
}

impl<T, E> OrRegistry for Result<T, E> {
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
                Some(value) => Ok(value),
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

impl<T> UnwrapOrRegistry for Option<T> {
    type Item = T;
    #[inline]
    fn unwrap_or_registry(
        self,
        registry: impl RegistryAccess<Item = Self::Item>,
        key: &NamespacedKey,
    ) -> Self::Item {
        match self {
            Some(value) => value,
            None => registry.get(key).unwrap(),
        }
    }
}

impl<T, E> UnwrapOrRegistry for Result<T, E> {
    type Item = T;
    #[inline]
    fn unwrap_or_registry(
        self,
        registry: impl RegistryAccess<Item = Self::Item>,
        key: &NamespacedKey,
    ) -> Self::Item {
        match self {
            Ok(value) => value,
            Err(_err) => registry.get(key).unwrap(),
        }
    }
}

pub trait RegistryAccess {
    type Item;
    fn get(&self, key: &NamespacedKey) -> Option<Self::Item>;
}

#[derive(Resource)]
pub struct Registry<T> {
    entries: HashMap<NamespacedKey, T>,
}

impl<T> Default for Registry<T> {
    fn default() -> Self {
        Self {
            entries: Default::default(),
        }
    }
}

impl<T> Registry<T> {
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
    pub fn register(&mut self, key: NamespacedKey, entry: T) -> Result<(), RegistryError> {
        if self.entries.contains_key(&key) {
            return Err(RegistryError::EntryAlreadyExistsError { key });
        }
        self.entries.insert(key, entry);
        Ok(())
    }
}

impl<'registry, T> RegistryAccess for &'registry Registry<T> {
    type Item = &'registry T;
    fn get(&self, key: &NamespacedKey) -> Option<Self::Item> {
        (self as &Registry<T>).get(key)
    }
}

#[derive(Resource)]
pub struct DynamicRegistry<T: ?Sized + Send + Sync + 'static> {
    entries: HashMap<NamespacedKey, Box<dyn Any + Send + Sync>>,
    _marker: PhantomData<T>,
}

impl<T: ?Sized + Send + Sync + 'static> Default for DynamicRegistry<T> {
    fn default() -> Self {
        Self {
            entries: Default::default(),
            _marker: PhantomData,
        }
    }
}

impl<T: ?Sized + Send + Sync + 'static> DynamicRegistry<T> {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn contains(&self, key: &NamespacedKey) -> bool {
        self.entries.contains_key(key)
    }
    pub fn get(&self, key: &NamespacedKey) -> Option<&T> {
        self.entries
            .get(key)
            .map(|boxed| *boxed.downcast_ref().unwrap())
    }
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.entries
            .values()
            .map(|boxed| *boxed.downcast_ref().unwrap())
    }
    pub fn register(&mut self, key: NamespacedKey, entry: &'static T) -> Result<(), RegistryError> {
        if self.entries.contains_key(&key) {
            return Err(RegistryError::EntryAlreadyExistsError { key });
        }
        self.entries.insert(key, Box::new(entry));
        Ok(())
    }
}

impl<'registry, T: ?Sized + Send + Sync> RegistryAccess for &'registry DynamicRegistry<T> {
    type Item = &'registry T;
    fn get(&self, key: &NamespacedKey) -> Option<Self::Item> {
        (self as &DynamicRegistry<T>).get(key)
    }
}

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("An entry with such a key already exists: {key}")]
    EntryAlreadyExistsError { key: NamespacedKey },
}
