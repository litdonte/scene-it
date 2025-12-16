use crate::utils;

// Until I move errors into their own module, they'll live in the module for which the error represents
#[derive(Debug)]
pub enum TitleError {
    EmptyTitle,
    TooLong,
    ContainsControlChars,
}

/// Represent the title of the user's story.
/// By default, the title is 'Untitled Storyboard',
/// unless provided during storyboard setup.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Title(String);

impl Default for Title {
    fn default() -> Self {
        Self(String::from("Untitled Storyboard")) // TODO: Add timestamp to default title
    }
}

impl Title {
    pub fn new(input: &str) -> Result<Self, TitleError> {
        let trimmed = utils::trim_input(input);

        if trimmed.is_empty() {
            return Err(TitleError::EmptyTitle);
        }

        if trimmed.chars().count() > 100 {
            return Err(TitleError::TooLong);
        }

        if trimmed.chars().any(|c| c.is_control()) {
            return Err(TitleError::ContainsControlChars);
        }

        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        return &self.0;
    }
}

impl TryFrom<&str> for Title {
    type Error = TitleError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
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

    #[test]
    fn creating_title_with_invalid_name_returns_default_title() {}
}
