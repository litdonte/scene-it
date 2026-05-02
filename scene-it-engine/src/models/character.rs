use crate::{
    models::{Id, metadata::Metadata},
    utils::{InputError, validate_input},
};
use serde::{Deserialize, Serialize};
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct CharacterName(String);

impl CharacterName {
    pub fn new(input: &str) -> Result<Self, InputError> {
        Ok(Self(validate_input(input, Some(100))?))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Character {
    id: Id<Self>,
    name: CharacterName,
    metadata: Metadata,
}

impl Character {
    pub fn new(name: CharacterName) -> Self {
        Self {
            id: Id::new(),
            name,
            metadata: Metadata::new(),
        }
    }

    pub fn id(&self) -> Id<Self> {
        self.id.clone()
    }
}

#[cfg(test)]
mod tests {}
