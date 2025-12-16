use crate::models::scene_elements::{action::SceneAction, dialogue::Dialogue};

pub mod action;
pub mod dialogue;
pub mod heading;

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
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
