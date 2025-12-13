use crate::utils::NamespacedKey;
use bevy::prelude::*;
use std::any::Any;
use std::collections::HashMap;
use std::marker::PhantomData;
use thiserror::Error;

#[derive(Default, Resource)]
pub struct Registry<T> {
    entries: HashMap<NamespacedKey, T>,
}

impl<T> Registry<T> {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
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
    pub fn contains(&self, key: &NamespacedKey) -> bool {
        self.entries.contains_key(key)
    }
    pub fn get(&self, key: &NamespacedKey) -> Option<&Box<T>> {
        self.entries
            .get(key)
            .map(|boxed| boxed.downcast_ref().unwrap())
    }
    pub fn iter(&self) -> impl Iterator<Item = &Box<T>> {
        self.entries
            .values()
            .map(|boxed| boxed.downcast_ref().unwrap())
    }
    pub fn register(&mut self, key: NamespacedKey, entry: &'static T) -> Result<(), RegistryError> {
        if self.entries.contains_key(&key) {
            return Err(RegistryError::EntryAlreadyExistsError { key });
        }
        self.entries.insert(key, Box::new(entry));
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("An entry with such a key already exists: {key}")]
    EntryAlreadyExistsError { key: NamespacedKey },
}
