use serde::{Deserialize, Serialize};
use std::ops::Deref;

use crate::utils::{InputError, validate_input};

/// Represent the title of the user's story.
/// By default, the title is 'Untitled Storyboard',
/// unless provided during storyboard setup.
#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
pub struct Title(String);

impl Default for Title {
    fn default() -> Self {
        Self(String::from("Untitled Storyboard")) // TODO: Add timestamp to default title
    }
}

impl Title {
    pub fn new(input: &str) -> Result<Self, InputError> {
        Ok(Self(validate_input(input, Some(100))?))
    }

    pub fn as_str(&self) -> &str {
        return &self.0;
    }
}

impl TryFrom<&str> for Title {
    type Error = InputError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl Deref for Title {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

#[cfg(test)]
mod tests {
    use crate::models::title::Title;

    #[test]
    fn creating_title_with_valid_name_works() {
        // Arrange & Act
        let input = "Scott Pilgrim      vs.     The World";
        let title = Title::new(input);
        // Assert
        assert_eq!("Scott Pilgrim vs. The World", title.unwrap().as_str())
    }
}
