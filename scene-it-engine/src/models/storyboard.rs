use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::models::{
    Id, author::Author, character::Character, metadata::Metadata, narrative::Narrative,
    summary::Summary, title::Title,
};

/// Represents the different types of script formats available.
///
/// Examples include:
///
/// - Teleplay
/// - Screenplay
/// - Half-hour Sitcom
/// - Novel
#[derive(Serialize, Deserialize, Clone)]
pub enum StoryTemplate {
    /// A script formatted for television production.
    Teleplay,
    /// A script formatted for film production.
    Screenplay,
    /// A script formatted for a half-hour sitcom, following its comedic beat structure.
    HalfHourSitcom,
    /// A prose narrative formatted as a novel.
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
#[derive(Serialize, Deserialize)]
pub struct Storyboard {
    /// The working title of the story, if one has been set.
    title: Option<Title>,
    /// The authors attached to the storyboard, keyed by ID.
    authors: HashMap<Id<Author>, Author>,
    /// The characters attached to the storyboard, keyed by ID.
    characters: HashMap<Id<Character>, Character>,
    /// The scenes and their relationships that make up the story.
    narrative: Narrative,
    /// The script format the story is being written for, if one has been selected.
    template: Option<StoryTemplate>,
    /// A summary of the story.
    summary: Summary,
    /// Bookkeeping metadata (e.g. creation and modification timestamps) for the storyboard.
    metadata: Metadata,
}

impl Storyboard {
    /// Returns the storyboard's title, if one has been set.
    pub fn title(&self) -> &Option<Title> {
        &self.title
    }

    /// Returns all authors attached to the storyboard.
    pub fn authors(&self) -> Vec<&Author> {
        self.authors.values().collect()
    }

    /// Returns all characters attached to the storyboard.
    pub fn characters(&self) -> Vec<&Character> {
        self.characters.values().collect()
    }

    /// Returns the storyboard's selected story template, if one has been chosen.
    pub fn template(&self) -> &Option<StoryTemplate> {
        &self.template
    }

    /// Returns the storyboard's summary.
    pub fn summary(&self) -> &Summary {
        &self.summary
    }

    /// Returns the storyboard's metadata.
    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }

    /// Sets or replaces the storyboard title.
    ///
    /// This overwrites any existing title. Titles are optional and may be
    /// added, removed, or changed at any point during storyboard development.
    pub fn update_title(&mut self, title: Title) {
        self.title = Some(title);
    }

    /// Removes the storyboard title, returning it to an unnamed state.
    ///
    /// This does not affect scenes, characters, or metadata.
    pub fn clear_title(&mut self) {
        self.title = None;
    }

    /// Sets or replaces the active story template.
    ///
    /// The template determines formatting rules and structural expectations
    /// (e.g. screenplay vs. novel), but does not immediately modify scene data.
    pub fn update_template(&mut self, template: StoryTemplate) {
        self.template = Some(template);
    }

    /// Clears the currently selected story template.
    ///
    /// After clearing, the storyboard has no enforced formatting or structure
    /// until a new template is selected.
    pub fn clear_template(&mut self) {
        self.template = None;
    }

    /// Adds an author to the storyboard.
    ///
    /// If an author with the same ID already exists, it will be replaced.
    pub fn add_author(&mut self, author: Author) {
        self.authors.insert(author.id(), author);
    }

    /// Removes an author from the storyboard by ID.
    ///
    /// Removing an author does not affect scenes or other storyboard data.
    pub fn remove_author(&mut self, author_id: &Id<Author>) {
        self.authors.remove(author_id);
    }

    /// Adds a character to the storyboard.
    ///
    /// If a character with the same ID already exists, it will be replaced.
    pub fn add_character(&mut self, character: Character) {
        self.characters.insert(character.id(), character);
    }
}

impl Default for Storyboard {
    /// Creates an empty `Storyboard` with no title, authors, characters, or
    /// template, and a fresh narrative, summary, and metadata.
    fn default() -> Self {
        Self {
            title: None,
            authors: HashMap::new(),
            characters: HashMap::new(),
            narrative: Narrative::default(),
            template: None,
            summary: Summary::default(),
            metadata: Metadata::new(),
        }
    }
}
