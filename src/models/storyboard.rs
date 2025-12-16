use std::collections::HashMap;

use crate::models::{
    Id, author::Author, character::Character, metadata::Metadata, scene::Scene, title::Title,
};

/// Represents the different types of script formats available.
///
/// Examples include:
///
/// - Teleplay
/// - Screenplay
/// - Half-hour Sitcom
/// - Novel
pub enum StoryTemplate {
    Teleplay,
    Screenplay,
    HalfHourSitcom,
    Novel,
}

/// The `Storyboard` is the project workbench and packages all of the story details.
///
/// From the storyboard, a user can:
/// - Add, edit, or remove a `Title`
/// - Create, edit, and delete a `Scene`
/// - Create, edit, and delete a `Character`
/// - Select and update the `StoryTemplate`
/// - Add and remove an `Author`
/// - Generate a story outline
pub struct Storyboard {
    title: Option<Title>,
    authors: HashMap<Id<Author>, Author>,
    scene_bank: HashMap<Id<Scene>, Scene>,
    characters: HashMap<Id<Character>, Character>,
    template: Option<StoryTemplate>,
    metadata: Metadata,
}

impl Storyboard {
    pub fn update_title(&mut self, title: Title) {
        self.title = Some(title);
    }

    pub fn clear_title(&mut self) {
        self.title = None;
    }

    pub fn update_template(&mut self, template: StoryTemplate) {
        self.template = Some(template);
    }

    pub fn clear_template(&mut self) {
        self.template = None;
    }

    pub fn add_author(&mut self, author: Author) {
        self.authors.insert(author.id(), author);
    }

    pub fn remove_author(&mut self, author_id: &Id<Author>) {
        self.authors.remove(author_id);
    }

    pub fn add_scene(&mut self, scene: Scene) {
        self.scene_bank.insert(scene.id(), scene);
    }

    pub fn add_character(&mut self, character: Character) {
        self.characters.insert(character.id(), character);
    }
}

impl Default for Storyboard {
    fn default() -> Self {
        Self {
            title: None,
            authors: HashMap::new(),
            scene_bank: HashMap::new(),
            characters: HashMap::new(),
            template: None,
            metadata: Metadata::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::models::storyboard::Storyboard;

    #[test]
    fn creating_storyboard_works() {
        // Arrange & Act
        let sb = Storyboard::default();
        // Assert
        assert_eq!(sb.title, None);
    }
}
