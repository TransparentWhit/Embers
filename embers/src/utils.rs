pub mod assets;

use bevy::asset::uuid::Uuid;
use bevy::prelude::*;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::hash::{BuildHasherDefault, DefaultHasher};
use std::result::Result;
use std::sync::LazyLock;
use thiserror::Error;

pub trait Marker: Clone + Send + Sync + 'static {}

impl<T: Clone + Send + Sync + 'static> Marker for T {}

pub type ConstHashMap<K, V> = HashMap<K, V, BuildHasherDefault<DefaultHasher>>;
pub const fn const_hash_map<K, V>() -> ConstHashMap<K, V> {
    HashMap::with_hasher(BuildHasherDefault::new())
}

pub type ConstHashSet<T> = HashSet<T, BuildHasherDefault<DefaultHasher>>;
pub const fn const_hash_set<T>() -> ConstHashSet<T> {
    HashSet::with_hasher(BuildHasherDefault::new())
}

pub trait Named {
    fn name(&self) -> &str;
}
pub trait UniquelyIdentified {
    fn unique_id(&self) -> &Uuid;
}
pub trait Namespaced {
    fn namespace(&self) -> &str;
}
pub trait Keyed {
    fn key(&self) -> &NamespacedKey;
}

impl<T> Keyed for (NamespacedKey, T) {
    fn key(&self) -> &NamespacedKey {
        &self.0
    }
}

#[derive(Debug, Error)]
pub enum IllegalNamespacedKeyError {
    #[error("Invalid namespace: {namespace}")]
    IllegalNamespaceError { namespace: String },
    #[error("Invalid key: {key}")]
    IllegalKeyError { key: String },
    #[error("Invalid namespaced key: {namespaced_key}")]
    IllegalNamespacedKeyError { namespaced_key: String },
}

#[derive(Component, Clone, Debug, Eq, Hash, PartialEq)]
pub struct NamespacedKey {
    namespaced_key: String,
    separator_index: usize,
}
pub static NAMESPACE_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\w+)$").unwrap());
pub static KEY_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([\w/]+)$").unwrap());
pub static NAMESPACED_KEY_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"^(?P<namespace>\w+){}(?P<key>[\w/]+)$",
        NamespacedKey::SEPARATOR
    ))
    .unwrap()
});
impl NamespacedKey {
    pub const SEPARATOR: &'static str = ":";
    const SEPARATOR_LEN: usize = Self::SEPARATOR.len();
    pub(crate) const EMBERS_NAMESPACE: &'static str = "embers";
    fn new_internal(namespace: &str, key: &str) -> Self {
        Self {
            namespaced_key: format!(
                "{}{separator}{}",
                namespace,
                key,
                separator = Self::SEPARATOR
            ),
            separator_index: namespace.len(),
        }
    }
    pub fn new<'namespace, 'key>(
        namespace: impl Into<&'namespace str>,
        key: impl Into<&'key str>,
    ) -> Self {
        let namespace = namespace.into();
        assert!(
            NAMESPACE_PATTERN.is_match(namespace),
            "Invalid namespace: {}",
            namespace
        );
        let key = key.into();
        assert!(KEY_PATTERN.is_match(key), "Invalid key: {}", key);
        Self::new_internal(namespace, key)
    }
    #[inline]
    pub fn new_namespaced<'key>(namespaced: &impl Namespaced, key: impl Into<&'key str>) -> Self {
        Self::new(namespaced.namespace(), key)
    }
    #[inline]
    pub(crate) fn new_embers(key: &str) -> Self {
        Self::new(Self::EMBERS_NAMESPACE, key)
    }
    pub fn try_from_with<'value, 'default_namespace>(
        value: impl Into<&'value str>,
        default_namespace: impl Into<&'default_namespace str>,
    ) -> Result<Self, IllegalNamespacedKeyError> {
        let value = value.into();
        let namespaced = Self::try_from(value);
        if namespaced.is_ok() {
            return namespaced;
        }
        if !KEY_PATTERN.is_match(value) {
            return Err(IllegalNamespacedKeyError::IllegalKeyError {
                key: value.to_string(),
            });
        }
        let default_namespace = default_namespace.into();
        if !NAMESPACE_PATTERN.is_match(default_namespace) {
            return Err(IllegalNamespacedKeyError::IllegalNamespaceError {
                namespace: default_namespace.to_string(),
            });
        }
        Ok(Self::new_internal(default_namespace, value))
    }
    #[inline]
    pub fn try_from_with_namespaced<'value>(
        value: impl Into<&'value str>,
        default_namespace: &impl Namespaced,
    ) -> Result<Self, IllegalNamespacedKeyError> {
        Self::try_from_with(value, default_namespace.namespace())
    }
    #[inline]
    pub(crate) fn try_from_with_embers<'value>(
        value: impl Into<&'value str>,
    ) -> Result<Self, IllegalNamespacedKeyError> {
        Self::try_from_with(value, Self::EMBERS_NAMESPACE)
    }
    pub fn key(&self) -> &str {
        &self.namespaced_key[(self.separator_index + Self::SEPARATOR_LEN)..]
    }
}
impl Namespaced for NamespacedKey {
    fn namespace(&self) -> &str {
        &self.namespaced_key[..self.separator_index]
    }
}
impl fmt::Display for NamespacedKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.namespaced_key)
    }
}
impl From<NamespacedKey> for String {
    fn from(value: NamespacedKey) -> Self {
        value.namespaced_key
    }
}
impl TryFrom<&str> for NamespacedKey {
    type Error = IllegalNamespacedKeyError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match NAMESPACED_KEY_PATTERN.captures(value) {
            Some(captures) => Ok(Self::new_internal(&captures["namespace"], &captures["key"])),
            None => Err(IllegalNamespacedKeyError::IllegalNamespacedKeyError {
                namespaced_key: value.to_string(),
            }),
        }
    }
}
