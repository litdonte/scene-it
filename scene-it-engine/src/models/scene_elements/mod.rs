use crate::models::scene_elements::{action::SceneAction, dialogue::Dialogue};
use serde::{Deserialize, Serialize};

pub mod action;
pub mod dialogue;
pub mod heading;

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum SceneElement {
    Action(SceneAction),
    Dialogue(Dialogue),
}

pub enum SceneElementError {
    EmptyHeadingLocation,
    EmptySceneAction,
    EmptyDialogueText,
    EmptyParenthetical,
    ContainsControlChars,
}
