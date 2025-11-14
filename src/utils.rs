use std::fmt;
use bevy::asset::uuid::Uuid;
use bevy::prelude::*;

#[macro_export]
macro_rules! identify {
    ($identified: ty, $identifier: ident) => {
        impl core::cmp::PartialEq for $identified {
            fn eq(&self, other: &Self) -> bool {
                self.$identifier() == other.$identifier()
            }
        }
        impl core::cmp::Eq for $identified {}
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

#[derive(Component, Clone, Debug, Eq, Hash, PartialEq)]
pub struct NamespacedKey {
    namespaced_key: String,
    separator_index: usize,
}
impl NamespacedKey {
    pub const SEPARATOR: &'static str = ":";
    const SEPARATOR_LEN: usize = Self::SEPARATOR.len();
    pub fn new(namespace: &str, key: &str) -> Self {
        Self {
            namespaced_key: format!("{}{separator}{}", namespace, key, separator = Self::SEPARATOR),
            separator_index: namespace.len(),
        }
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
