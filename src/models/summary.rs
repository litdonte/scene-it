use std::ops::Deref;

use crate::utils;

// Until I move errors into their own module, they'll live in the module for which the error represents
#[derive(Debug)]
pub enum SummaryError {
    EmptySummary,
    ContainsControlChars,
}

/// Represent the summary of the user's story.
/// By default, the summary is empty,
/// unless provided during storyboard setup.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Summary(String);

impl Default for Summary {
    fn default() -> Self {
        Self(String::new()) // TODO: Add timestamp to default title
    }
}

impl Summary {
    pub fn new(input: &str) -> Result<Self, SummaryError> {
        let trimmed = utils::trim_input(input);

        if trimmed.is_empty() {
            return Err(SummaryError::EmptySummary);
        }

        if trimmed.chars().any(|c| c.is_control()) {
            return Err(SummaryError::ContainsControlChars);
        }

        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        return &self.0;
    }
}

impl TryFrom<&str> for Summary {
    type Error = SummaryError;

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

    #[test]
    fn creating_title_with_invalid_name_returns_default_title() {}
}
