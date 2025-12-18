use crate::{models::scene_elements::SceneElementError, utils};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SceneAction(String);

impl SceneAction {
    pub fn new(input: &str) -> Result<Self, SceneElementError> {
        let trimmed = utils::trim_input(input);

        if trimmed.is_empty() {
            return Err(SceneElementError::EmptySceneAction);
        }

        if trimmed.chars().any(|c| c.is_control()) {
            return Err(SceneElementError::ContainsControlChars);
        }

        Ok(Self(trimmed))
    }
}
