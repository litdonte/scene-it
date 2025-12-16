mod author;
mod character;
mod metadata;
mod scene;
mod scene_elements;
mod storyboard;
mod title;

use std::{fmt, marker::PhantomData};
use uuid::Uuid;

#[derive(Debug, Clone, Hash, PartialEq, Eq, Copy)]
struct Id<T> {
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
