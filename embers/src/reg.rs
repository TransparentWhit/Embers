use crate::utils::{Keyed, NamespacedKey};
use bevy::prelude::*;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::ops::Deref;
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
            None => registry.get_cloned(key),
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
            Err(err) => registry.get_cloned(key).ok_or(err),
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
            None => registry.get_cloned(key).unwrap(),
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
            Err(_err) => registry.get_cloned(key).unwrap(),
        }
    }
}

pub trait RegistryAccess {
    type Item: ?Sized;
    fn contains(&self, key: &NamespacedKey) -> bool;
    fn get(&self, key: &NamespacedKey) -> Option<&Self::Item>;
    fn get_cloned(&self, key: &NamespacedKey) -> Option<Self::Item>
    where
        Self::Item: Clone;
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

pub type Reg<'world, T> = Res<'world, Registry<T>>;

pub type RegMut<'world, T> = ResMut<'world, Registry<T>>;

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
    #[inline]
    pub fn get_cloned(&self, key: &NamespacedKey) -> Option<T>
    where
        T: Clone,
    {
        self.get(key).cloned()
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
    #[inline]
    pub fn register_keyed(&mut self, value: T) -> Result<(), RegistryError>
    where
        T: Keyed,
    {
        self.register(value.key().clone(), value)
    }
}

macro_rules! impl_registry_access {
    ($type_: ty, $get_registry: ident) => {
        impl<T: Send + Sync + 'static> RegistryAccess for $type_ {
            type Item = T;
            #[inline]
            fn contains(&self, key: &NamespacedKey) -> bool {
                self.$get_registry().contains(key)
            }
            #[inline]
            fn get(&self, key: &NamespacedKey) -> Option<&Self::Item> {
                self.$get_registry().get(key)
            }
            #[inline]
            fn get_cloned(&self, key: &NamespacedKey) -> Option<Self::Item>
            where
                Self::Item: Clone,
            {
                self.$get_registry().get_cloned(key)
            }
            #[inline]
            fn iter(&self) -> impl Iterator<Item = &Self::Item> {
                self.$get_registry().iter()
            }
            #[inline]
            fn iter_entries(&self) -> impl Iterator<Item = (&NamespacedKey, &Self::Item)> {
                self.$get_registry().iter_entries()
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
    #[inline]
    pub fn get_cloned(&self, key: &NamespacedKey) -> Option<T>
    where
        T: Clone,
    {
        self.get(key).cloned()
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
    #[inline]
    pub fn register_keyed(&mut self, value: T) -> Result<(), RegistryError>
    where
        T: Keyed + Sized,
    {
        self.register(value.key().clone(), value)
    }
    #[inline]
    pub fn register_boxed_keyed(&mut self, boxed_value: Box<T>) -> Result<(), RegistryError>
    where
        T: Keyed,
    {
        self.register_boxed(boxed_value.key().clone(), boxed_value)
    }
}

macro_rules! impl_dyn_registry_access {
    ($type_: ty, $get_registry: ident) => {
        impl<T: ?Sized + Send + Sync + 'static> RegistryAccess for $type_ {
            type Item = T;
            #[inline]
            fn contains(&self, key: &NamespacedKey) -> bool {
                self.$get_registry().contains(key)
            }
            #[inline]
            fn get(&self, key: &NamespacedKey) -> Option<&Self::Item> {
                self.$get_registry().get(key)
            }
            #[inline]
            fn get_cloned(&self, key: &NamespacedKey) -> Option<Self::Item>
            where
                Self::Item: Clone,
            {
                self.$get_registry().get_cloned(key)
            }
            #[inline]
            fn iter(&self) -> impl Iterator<Item = &Self::Item> {
                self.$get_registry().iter()
            }
            #[inline]
            fn iter_entries(&self) -> impl Iterator<Item = (&NamespacedKey, &Self::Item)> {
                self.$get_registry().iter_entries()
            }
        }
    };
}

impl_dyn_registry_access!(&DynReg<'_, T>, as_ref);
impl_dyn_registry_access!(&DynRegMut<'_, T>, as_ref);
impl_dyn_registry_access!(&DynamicRegistry<T>, deref);

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
