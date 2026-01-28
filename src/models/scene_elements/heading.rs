use crate::{models::scene_elements::SceneElementError, utils};
use serde::{Deserialize, Serialize};

#[derive(Debug, Hash, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum CameraLocation {
    Interior,
    Exterior,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct SceneLocation(String);

impl SceneLocation {
    pub fn new(input: &str) -> Result<Self, SceneElementError> {
        let trimmed = utils::trim_input(input);

        if trimmed.is_empty() {
            return Err(SceneElementError::EmptyHeadingLocation);
        }

        if trimmed.chars().any(|c| c.is_control()) {
            return Err(SceneElementError::ContainsControlChars);
        }

        Ok(Self(trimmed))
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
