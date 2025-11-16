pub mod assets;

use std::fmt;
use std::result::Result;
use std::sync::LazyLock;
use bevy::asset::uuid::Uuid;
use bevy::prelude::*;
use regex::Regex;
use thiserror::Error;

#[macro_export]
macro_rules! identify {
    ($identified: ty, $identifier: ident) => {
        impl std::cmp::PartialEq for $identified {
            fn eq(&self, other: &Self) -> bool {
                self.$identifier() == other.$identifier()
            }
        }
        impl std::cmp::Eq for $identified {}
        impl std::hash::Hash for $identified {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                self.$identifier().hash(state);
            }
        }
    };
}
#[macro_export]
macro_rules! uniquely_identify {
    ($uniquely_identified: ty) => {
        $crate::identify!($uniquely_identified, unique_id);
    };
}
#[macro_export]
macro_rules! key_identify {
    ($key_identified: ty) => {
        $crate::identify!($key_identified, key);
    };
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
pub static KEY_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\w+)$").unwrap());
pub static NAMESPACED_KEY_PATTERN: LazyLock<Regex> = LazyLock::new(|| Regex::new(&format!(r"^(?P<namespace>\w+){}(?P<key>\w+)$", NamespacedKey::SEPARATOR)).unwrap());
impl NamespacedKey {
    pub const SEPARATOR: &'static str = ":";
    const SEPARATOR_LEN: usize = Self::SEPARATOR.len();
    pub(crate) const EMBERS_NAMESPACE: &'static str = "embers";
    fn new_internal(namespace: &str, key: &str) -> Self {
        Self {
            namespaced_key: format!("{}{separator}{}", namespace, key, separator = Self::SEPARATOR),
            separator_index: namespace.len(),
        }
    }
    pub fn new<'a, 'b>(namespace: impl Into<&'a str>, key: impl Into<&'b str>) -> Self {
        let namespace = namespace.into();
        assert!(NAMESPACE_PATTERN.is_match(namespace), "Invalid namespace: {}", namespace);
        let key = key.into();
        assert!(KEY_PATTERN.is_match(key), "Invalid key: {}", key);
        Self::new_internal(namespace, key)
    }
    #[inline]
    pub fn new_namespaced<'a>(namespaced: &impl Namespaced, key: impl Into<&'a str>) -> Self {
        Self::new(namespaced.namespace(), key)
    }
    #[inline]
    pub(crate) fn new_embers(key: &str) -> Self {
        Self::new(Self::EMBERS_NAMESPACE, key)
    }
    pub fn try_from_with<'a, 'b>(value: impl Into<&'a str>, default_namespace: impl Into<&'b str>) -> Result<Self, IllegalNamespacedKeyError> {
        let value = value.into();
        let namespaced = Self::try_from(value);
        if (namespaced.is_ok()) {
            return namespaced;
        }
        if (!KEY_PATTERN.is_match(value)) {
            return Err(IllegalNamespacedKeyError::IllegalKeyError { key: value.to_string() });
        }
        let default_namespace = default_namespace.into();
        if (!NAMESPACE_PATTERN.is_match(default_namespace)) {
            return Err(IllegalNamespacedKeyError::IllegalNamespaceError { namespace: default_namespace.to_string() });
        }
        Ok(Self::new_internal(default_namespace, value))
    }
    #[inline]
    pub fn try_from_with_namespaced<'a>(value: impl Into<&'a str>, default_namespace: &impl Namespaced) -> Result<Self, IllegalNamespacedKeyError> {
        Self::try_from_with(value, default_namespace.namespace())
    }
    #[inline]
    pub(crate) fn try_from_with_embers<'a>(value: impl Into<&'a str>) -> Result<Self, IllegalNamespacedKeyError> {
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
impl Into<String> for NamespacedKey {
    fn into(self) -> String {
        self.namespaced_key
    }
}
impl TryFrom<&str> for NamespacedKey {
    type Error = IllegalNamespacedKeyError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match NAMESPACED_KEY_PATTERN.captures(value) {
            Some(captures) => Ok(Self::new_internal(&captures["namespace"], &captures["key"])),
            None => Err(IllegalNamespacedKeyError::IllegalNamespacedKeyError { namespaced_key: value.to_string() }),
        }
    }
}
