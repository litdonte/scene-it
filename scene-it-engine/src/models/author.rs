use crate::{
    models::{Id, metadata::Metadata},
    utils::{InputError, validate_input},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct AuthorName(String);

impl AuthorName {
    pub fn new(input: &str) -> Result<Self, InputError> {
        Ok(Self(validate_input(input, Some(100))?))
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
