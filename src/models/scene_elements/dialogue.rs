use crate::{
    models::{
        Id, character::Character, metadata::Metadata, scene::Scene,
        scene_elements::SceneElementError,
    },
    utils::{self, trim_input},
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Parenthetical(String);

impl Parenthetical {
    pub fn new(input: &str) -> Result<Self, SceneElementError> {
        let trimmed = trim_input(input);

        if trimmed.is_empty() {
            return Err(SceneElementError::EmptyParenthetical);
        }

        Ok(Self(trimmed))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DialogueText(String);

impl DialogueText {
    pub fn new(input: &str) -> Result<Self, SceneElementError> {
        let trimmed = utils::trim_input(input);

        if trimmed.is_empty() {
            return Err(SceneElementError::EmptyDialogueText);
        }

        if trimmed.chars().any(|c| c.is_control()) {
            return Err(SceneElementError::ContainsControlChars);
        }

        Ok(Self(trimmed))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum DialogueBlock {
    Text(DialogueText),
    Parenthetical(Parenthetical),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Dialogue {
    id: Id<Self>,
    scene: Id<Scene>,
    speaker: Id<Character>,
    content: Vec<DialogueBlock>,
    metadata: Metadata,
}

impl Dialogue {
    pub fn new(scene: Id<Scene>, speaker: Id<Character>) -> Self {
        Self {
            id: Id::new(),
            scene,
            speaker,
            content: Vec::new(),
            metadata: Metadata::new(),
        }
    }

    pub fn add_dialogue_block(&mut self, block: DialogueBlock) {
        self.content.push(block);
    }
}
