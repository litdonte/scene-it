use serde::{Deserialize, Serialize};
use std::ops::Deref;

use crate::utils::{InputError, validate_input};

/// Represent the summary of the user's story.
/// By default, the summary is empty,
/// unless provided during storyboard setup.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Summary(String);

impl Default for Summary {
    fn default() -> Self {
        Self(String::new()) // TODO: Add timestamp to default title
    }
}

impl Summary {
    pub fn new(input: &str) -> Result<Self, InputError> {
        Ok(Self(validate_input(input, None)?))
    }

    pub fn as_str(&self) -> &str {
        return &self.0;
    }
}

impl TryFrom<&str> for Summary {
    type Error = InputError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl Deref for Summary {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

#[cfg(test)]
mod tests {
    use crate::models::summary::Summary;

    #[test]
    fn creating_title_with_valid_name_works() {
        // Arrange & Act
        let input = "Scott Pilgrim      vs.     The World";
        let title = Summary::new(input);
        // Assert
        assert_eq!("Scott Pilgrim vs. The World", title.unwrap().as_str())
    }
}
