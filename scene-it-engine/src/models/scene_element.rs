use crate::{
    models::{Id, character::Character, metadata::Metadata, scene::Scene},
    utils::{InputError, validate_input},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum SceneElement {
    Action(SceneAction),
    Dialogue(Dialogue),
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct SceneAction(String);

impl SceneAction {
    pub fn new(input: &str) -> Result<Self, InputError> {
        Ok(Self(validate_input(input, None)?))
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Parenthetical(String);

impl Parenthetical {
    pub fn new(input: &str) -> Result<Self, InputError> {
        Ok(Self(validate_input(input, Some(25))?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct DialogueText(String);

impl DialogueText {
    pub fn new(input: &str) -> Result<Self, InputError> {
        Ok(Self(validate_input(input, None)?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum DialogueBlock {
    Text(DialogueText),
    Parenthetical(Parenthetical),
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Hash, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum CameraLocation {
    Interior,
    Exterior,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct SceneLocation(String);

impl SceneLocation {
    pub fn new(input: &str) -> Result<Self, InputError> {
        Ok(Self(validate_input(input, None)?))
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum SceneTimeOfDay {
    Morning,
    Dawn,
    Day,
    Dusk,
    Evening,
    Night,
    Later,
    Continuous,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct SceneHeading {
    camera_location: CameraLocation,
    scene_location: SceneLocation,
    time_of_day: SceneTimeOfDay,
}

impl SceneHeading {
    pub fn new(
        camera_location: CameraLocation,
        scene_location: SceneLocation,
        time_of_day: SceneTimeOfDay,
    ) -> Self {
        Self {
            camera_location,
            scene_location,
            time_of_day,
        }
    }
}
