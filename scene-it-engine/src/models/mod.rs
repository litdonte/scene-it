mod author;
mod character;
mod metadata;
mod scene;
mod scene_elements;
mod scene_graph;
mod storyboard;
mod summary;
mod title;

pub use storyboard::Storyboard;

use serde::{Deserialize, Serialize};
use std::{
    fmt,
    hash::{Hash, Hasher},
    marker::PhantomData,
};
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub struct Id<T> {
    value: Uuid,
    _kind: PhantomData<T>,
}

impl<T> Id<T> {
    pub fn new() -> Self {
        Self {
            value: Uuid::new_v4(),
            _kind: PhantomData,
        }
    }

    pub fn uuid(&self) -> Uuid {
        self.value
    }
}

impl<T> fmt::Display for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl<T> From<Uuid> for Id<T> {
    fn from(uuid: Uuid) -> Self {
        Self {
            value: uuid,
            _kind: PhantomData,
        }
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T> Eq for Id<T> {}

impl<T> Hash for Id<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl<T> Serialize for Id<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.value.to_string())
    }
}

impl<'de, T> Deserialize<'de> for Id<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let value = Uuid::parse_str(&s).map_err(serde::de::Error::custom)?;
        Ok(Self {
            value,
            _kind: PhantomData,
        })
    }
}
