use crate::{
    models::{metadata::Metadata, Id},
    utils,
};
use serde::{Deserialize, Serialize};

pub enum AuthorError {
    EmptyName,
    NameTooLong,
    NameContainsControlChars,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct AuthorName(String);

impl AuthorName {
    pub fn new(input: &str) -> Result<Self, AuthorError> {
        let name = utils::trim_input(input);

        if name.is_empty() {
            return Err(AuthorError::EmptyName);
        }

        if name.len() > 100 {
            return Err(AuthorError::NameTooLong);
        }

        if name.chars().any(|c| c.is_control()) {
            return Err(AuthorError::NameContainsControlChars);
        }

        Ok(Self(name.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// Represents the profile of the Author of the story.
/// Currently, it only takes the name of the author as an argument.
///
/// TODO: Expand to include a full public/private profile with metadata.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Author {
    id: Id<Self>,
    name: AuthorName,
    metadata: Metadata,
}

impl Author {
    pub fn new(name: AuthorName) -> Self {
        Self {
            id: Id::new(),
            name,
            metadata: Metadata::new(),
        }
    }

    pub fn id(&self) -> Id<Self> {
        self.id.clone()
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }
}

#[cfg(test)]
mod tests {}
