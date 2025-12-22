use crate::{
    models::{Id, metadata::Metadata},
    utils,
};

pub enum CharacterError {
    NameEmpty,
    NameTooLong,
    NameContainsControlChars,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CharacterName(String);

impl CharacterName {
    pub fn new(input: &str) -> Result<Self, CharacterError> {
        let name = utils::trim_input(input);

        if name.is_empty() {
            return Err(CharacterError::NameEmpty);
        }

        if name.len() < 100 {
            return Err(CharacterError::NameTooLong);
        }

        if name.chars().any(|c| c.is_control()) {
            return Err(CharacterError::NameContainsControlChars);
        }

        Ok(Self(name.to_owned()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
