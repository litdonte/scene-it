use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::models::{
    Id,
    author::Author,
    character::Character,
    metadata::{HasMetadata, Metadata},
    scene::Scene,
    scene_graph::SceneGraph,
    title::Title,
};

pub enum StoryboardError {
    UnknownScene(Id<Scene>),
    InvalidMove {
        scene: Id<Scene>,
        from: Id<Scene>,
        dest: Id<Scene>,
    },
    CycleDetected(Id<Scene>, Id<Scene>),
    SceneNotInGraph(Id<Scene>),
}

/// Represents a structural change to a `Storyboard` caused by an operation on
/// the underlying `SceneGraph`.
///
/// `StoryboardUpdate` acts as a **change notification** rather than a command.
/// The `SceneGraph` produces updates describing *what changed*, and the
/// `Storyboard` decides *how to react* (e.g., touching metadata, invalidating
/// caches, triggering UI refreshes, etc.).
///
/// This separation ensures:
/// - The `SceneGraph` remains purely structural (IDs and relationships only)
/// - The `Storyboard` remains the single owner of scene data and metadata
/// - Side effects (timestamps, revision tracking) are centralized
///
/// Updates are intended to be:
/// - **Exhaustive**: every meaningful graph mutation emits one
/// - **Deterministic**: no hidden side effects
/// - **Composable**: callers can pattern-match and react selectively
pub enum StoryboardUpdate {
    Move {
        scene: Id<Scene>,
        from: Id<Scene>,
        dest: Id<Scene>,
    },
    SceneAdded(Id<Scene>),
    SceneSetAsRoot(Id<Scene>),
    LinkedScenes {
        from: Id<Scene>,
        dest: Id<Scene>,
    },
    SceneDeleted(Id<Scene>),
    EdgeDeleted {
        from: Id<Scene>,
        dest: Id<Scene>,
    },
}

/// Represents the different types of script formats available.
///
/// Examples include:
///
/// - Teleplay
/// - Screenplay
/// - Half-hour Sitcom
/// - Novel
#[derive(Serialize, Deserialize)]
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
#[derive(Serialize, Deserialize)]
pub struct Storyboard {
    title: Option<Title>,
    authors: HashMap<Id<Author>, Author>,
    scene_bank: HashMap<Id<Scene>, Scene>,
    characters: HashMap<Id<Character>, Character>,
    template: Option<StoryTemplate>,
    scene_graph: SceneGraph,
    metadata: Metadata,
}

impl Storyboard {
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

    /// Adds a new scene to the storyboard.
    ///
    /// This registers the scene in both the scene graph (for ordering and
    /// relationships) and the scene bank (for scene data storage).
    pub fn add_scene(&mut self, scene: Scene) {
        self.scene_graph.add_scene(&scene.id());
        self.scene_bank.insert(scene.id(), scene);
    }

    /// Moves a scene from one parent scene to another in the scene graph.
    ///
    /// This operation updates scene relationships only. If the move would
    /// create a cycle or otherwise violate graph constraints, an error is returned.
    /// On success, affected scenes have their metadata updated.
    pub fn move_scene(
        &mut self,
        scene: &Id<Scene>,
        from: &Id<Scene>,
        to: &Id<Scene>,
    ) -> Result<(), StoryboardError> {
        let graph_update = self.scene_graph.move_scene(scene, from, to)?;
        self.apply_update(graph_update);
        Ok(())
    }

    /// Applies a structural update emitted by the scene graph.
    ///
    /// This method synchronizes storyboard-owned data (such as scene metadata)
    /// with graph-level changes without duplicating graph logic.
    fn apply_update(&mut self, update: StoryboardUpdate) {
        match update {
            StoryboardUpdate::Move { scene, from, dest } => {
                self.update_metadata(&scene);
                self.update_metadata(&from);
                self.update_metadata(&dest);
            }
            StoryboardUpdate::SceneAdded(scene)
            | StoryboardUpdate::SceneSetAsRoot(scene)
            | StoryboardUpdate::SceneDeleted(scene) => {
                self.update_metadata(&scene);
            }
            StoryboardUpdate::LinkedScenes { from, dest }
            | StoryboardUpdate::EdgeDeleted { from, dest } => {
                self.update_metadata(&from);
                self.update_metadata(&dest);
            }
        }
    }

    /// Updates metadata for a scene by marking it as modified.
    ///
    /// This is typically called after structural changes such as moves,
    /// edge updates, or deletions.
    fn update_metadata(&mut self, scene: &Id<Scene>) {
        if let Some(scene) = self.scene_bank.get_mut(scene) {
            scene.touch();
        }
    }

    /// Removes a scene from the storyboard and its scene graph.
    ///
    /// This method performs a coordinated deletion across both ownership layers:
    ///
    /// - The scene is removed from the storyboard’s `scene_bank` (data ownership).
    /// - The scene is removed from the `SceneGraph` (structural relationships),
    ///   including:
    ///   - The scene node itself
    ///   - Any edges pointing *to* or *from* the scene
    ///   - Root references, if the scene was an entry point
    ///
    /// If the scene does not exist in the storyboard, this method is a no-op
    /// and returns `Ok(())`.
    ///
    /// # Errors
    ///
    /// Returns a [`StoryboardError`] if the scene exists in the storyboard but
    /// cannot be removed from the scene graph (for example, if the graph is in
    /// an inconsistent state).
    ///
    /// # Side Effects
    ///
    /// - Applies a [`StoryboardUpdate::SceneDeleted`] update
    /// - Touches metadata for affected scenes via `apply_update`
    ///
    /// # Examples
    ///
    /// ```rust
    /// storyboard.delete_scene(&scene_id)?;
    /// ```
    pub fn delete_scene(&mut self, scene: &Id<Scene>) -> Result<(), StoryboardError> {
        if let Some(scene) = self.scene_bank.remove(scene) {
            let graph_update = self.scene_graph.delete_scene(&scene.id())?;
            self.apply_update(graph_update);
        }
        Ok(())
    }

    /// Marks a scene as a root entry point in the scene graph.
    ///
    /// Root scenes represent valid starting points for story traversal.
    pub fn set_scene_as_root(&mut self, scene_id: &Id<Scene>) {
        self.scene_graph.add_root(scene_id);
    }

    /// Adds a character to the storyboard.
    ///
    /// If a character with the same ID already exists, it will be replaced.
    pub fn add_character(&mut self, character: Character) {
        self.characters.insert(character.id(), character);
    }

    /// Creates a directional link between two scenes.
    ///
    /// The `from` scene will be considered a predecessor of the `to` scene
    /// during traversal and linearization. Both scenes must already exist.
    pub fn link_scenes(&mut self, from: &Id<Scene>, to: &Id<Scene>) -> Result<(), StoryboardError> {
        if !self.scene_bank.contains_key(&from) {
            return Err(StoryboardError::UnknownScene(from.clone()));
        }

        if !self.scene_bank.contains_key(&to) {
            return Err(StoryboardError::UnknownScene(to.clone()));
        }

        let graph_update = self.scene_graph.add_edge(from, to);
        self.apply_update(graph_update);

        Ok(())
    }

    /// Removes a directed edge between two scenes in the scene graph.
    ///
    /// This operation disconnects `to` as a possible successor of `from`,
    /// without deleting either scene from the storyboard.
    ///
    /// # Errors
    ///
    /// Returns [`StoryboardError::UnknownScene`] if either `from` or `to`
    /// does not exist in the storyboard.
    ///
    /// Returns [`StoryboardError::SceneNotInGraph`] if either `from` or `to`
    /// does not exist in the scene graph.
    ///
    /// Returns a graph-level error if the edge does not exist or cannot
    /// be removed (for example, due to internal graph invariants).
    ///
    /// # Side Effects
    ///
    /// - Updates the scene graph structure
    /// - Applies metadata updates to the affected scenes via `apply_update`
    ///
    /// # Use Cases
    ///
    /// - Removing an optional or branching story path
    /// - Reworking story flow without deleting scenes
    /// - Allowing users to manually prune narrative branches
    pub fn unlink_scenes(
        &mut self,
        from: &Id<Scene>,
        to: &Id<Scene>,
    ) -> Result<(), StoryboardError> {
        if !self.scene_bank.contains_key(&from) {
            return Err(StoryboardError::UnknownScene(from.clone()));
        }

        if !self.scene_bank.contains_key(&to) {
            return Err(StoryboardError::UnknownScene(to.clone()));
        }

        let graph_update = self.scene_graph.delete_edge(from, to)?;
        self.apply_update(graph_update);
        Ok(())
    }

    /// Returns all scenes that are unreachable from any root scene.
    ///
    /// A standalone scene is defined as one that:
    /// - Is not a root scene
    /// - Is not reachable from any root via directed edges
    ///
    /// These scenes are considered *orphaned* in the narrative structure
    /// and will not appear in any linearized story output.
    ///
    /// # Returns
    ///
    /// A [`HashSet`] of scene IDs representing unreachable scenes.
    ///
    /// # Use Cases
    ///
    /// - Detecting unused or forgotten scenes
    /// - Providing UI warnings or cleanup suggestions
    /// - Helping users identify narrative dead ends
    pub fn standalone_scenes(&self) -> HashSet<Id<Scene>> {
        self.scene_graph.unreachable_scenes()
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
            scene_graph: SceneGraph::new(),
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
