use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::models::{
    Id,
    author::Author,
    character::Character,
    metadata::{HasMetadata, Metadata},
    scene::{Scene, SceneVariant},
    scene_graph::{SceneGraph, SceneGraphError, SceneGraphUpdate},
    summary::Summary,
    title::Title,
};

pub(crate) enum StoryboardError {
    UnknownScene(Id<Scene>),
    SceneGraph(SceneGraphError),
}

impl From<SceneGraphError> for StoryboardError {
    fn from(value: SceneGraphError) -> Self {
        Self::SceneGraph(value)
    }
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
pub(crate) enum StoryTemplate {
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
pub(crate) struct Storyboard {
    title: Option<Title>,
    authors: HashMap<Id<Author>, Author>,
    scene_bank: HashMap<Id<Scene>, Scene>,
    characters: HashMap<Id<Character>, Character>,
    template: Option<StoryTemplate>,
    scene_graph: SceneGraph,
    summary: Summary,
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
        for variant_id in scene.variant_ids() {
            self.scene_graph.add_variant(variant_id);
        }
        self.scene_bank.insert(scene.id(), scene);
    }

    /// Moves a scene from one parent scene to another in the scene graph.
    ///
    /// This operation updates scene relationships only. If the move would
    /// create a cycle or otherwise violate graph constraints, an error is returned.
    /// On success, affected scenes have their metadata updated.
    pub fn move_scene(
        &mut self,
        scene: &Id<SceneVariant>,
        src: &Id<SceneVariant>,
        to: &Id<SceneVariant>,
    ) -> Result<(), StoryboardError> {
        let graph_update = self.scene_graph.move_variant(scene, src, to)?;
        self.apply_scene_graph_update(graph_update);
        Ok(())
    }

    pub fn linearize_from<'a>(
        &'a self,
        root: &'a Id<SceneVariant>,
    ) -> impl Iterator<Item = &'a Scene> {
        let mut current = Some(root);
        let mut visited = HashSet::new();
        let mut order = Vec::new();

        while let Some(variant_id) = current {
            if !visited.insert(variant_id) {
                break;
            }

            if let Some(scene) = self.scene_bank.values().find(|s| s.has_variant(variant_id)) {
                order.push(scene);

                current = scene.variants().get(variant_id).and_then(|v| v.next());
            } else {
                eprintln!("Warning: variant ID {variant_id} found in graph but not in any scene");
            }
        }

        order.into_iter()
    }

    /// Applies a structural update emitted by the scene graph.
    ///
    /// This method synchronizes storyboard-owned data (such as scene metadata)
    /// with graph-level changes without duplicating graph logic.
    fn apply_scene_graph_update(&mut self, update: SceneGraphUpdate) {
        match update {
            SceneGraphUpdate::Move { variant, src, dest } => {
                self.update_metadata(&variant);
                self.update_metadata(&src);
                self.update_metadata(&dest);
            }
            SceneGraphUpdate::SceneVariantAdded(variant)
            | SceneGraphUpdate::SceneVariantSetAsRoot(variant)
            | SceneGraphUpdate::SceneVariantDeleted(variant) => {
                self.update_metadata(&variant);
            }
            SceneGraphUpdate::LinkedSceneVariants { src, dest }
            | SceneGraphUpdate::EdgeDeleted { src, dest } => {
                self.update_metadata(&src);
                self.update_metadata(&dest);
            }
        }
    }

    /// Updates metadata for a scene by marking it as modified.
    ///
    /// This is typically called after structural changes such as moves,
    /// edge updates, or deletions.
    fn update_metadata(&mut self, variant_id: &Id<SceneVariant>) {
        for scene in self.scene_bank.values_mut() {
            if scene.has_variant(variant_id) {
                scene.touch();
            }
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
    /// - Applies a [`SceneGraphUpdate::SceneVariantDeleted`] update
    /// - Touches metadata for affected scenes via `apply_scene_graph_update`
    ///
    /// # Examples
    ///
    pub fn delete_scene(&mut self, scene: &Id<Scene>) -> Result<(), StoryboardError> {
        if let Some(scene) = self.scene_bank.remove(scene) {
            for variant in scene.variant_ids() {
                let graph_update = self.scene_graph.delete_variant(variant)?;
                self.apply_scene_graph_update(graph_update);
            }
        }
        Ok(())
    }

    /// Marks a scene as a root entry point in the scene graph.
    ///
    /// Root scenes represent valid starting points for story traversal.
    pub fn set_variant_as_root(&mut self, variant_id: &Id<SceneVariant>) {
        self.scene_graph.add_root(variant_id);
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
    pub fn link_variants(
        &mut self,
        src: &Id<SceneVariant>,
        to: &Id<SceneVariant>,
    ) -> Result<(), StoryboardError> {
        let graph_update = self.scene_graph.add_edge(src, to);
        self.apply_scene_graph_update(graph_update);
        Ok(())
    }

    /// Removes a directed edge between two scenes in the scene graph.
    ///
    /// This operation disconnects `to` as a possible successor of `from`,
    /// without deleting either scene from the storyboard.
    ///
    /// # Errors
    ///
    /// Returns [`SceneGraphError::UnknownVariant`] if either `from` or `to`
    /// does not exist in the storyboard or the scene graph.
    ///
    /// Returns a graph-level error if the edge does not exist or cannot
    /// be removed (for example, due to internal graph invariants).
    ///
    /// # Side Effects
    ///
    /// - Updates the scene graph structure
    /// - Applies metadata updates to the affected scenes via `apply_scene_graph_update`
    ///
    /// # Use Cases
    ///
    /// - Removing an optional or branching story path
    /// - Reworking story flow without deleting scenes
    /// - Allowing users to manually prune narrative branches
    pub fn unlink_scenes(
        &mut self,
        src: &Id<SceneVariant>,
        dest: &Id<SceneVariant>,
    ) -> Result<(), StoryboardError> {
        if !self.scene_bank.values().any(|s| s.has_variant(src)) {
            return Err(SceneGraphError::UnknownVariant(src.clone()).into());
        }

        if !self.scene_bank.values().any(|s| s.has_variant(dest)) {
            return Err(SceneGraphError::UnknownVariant(dest.clone()).into());
        }

        let graph_update = self.scene_graph.delete_edge(src, dest)?;
        self.apply_scene_graph_update(graph_update);
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
        self.scene_graph.unreachable_variants();
        todo!()
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
            summary: Summary::default(),
            metadata: Metadata::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::models::{
        Id,
        scene::{Scene, SceneVariant},
        storyboard::Storyboard,
    };

    #[test]
    fn linearize_from_returns_scenes_in_order() {
        // Arrange
        let mut storyboard = Storyboard::default();

        let mut scene1 = Scene::new();
        let mut scene2 = Scene::new();
        let scene3 = Scene::new();

        // Get the active variant ID from each scene
        let variant1_id = scene1.active_variant().clone();
        let variant2_id = scene2.active_variant().clone();
        let variant3_id = scene3.active_variant().clone();

        // Wire up next pointers
        scene1
            .variants_mut()
            .get_mut(&variant1_id)
            .unwrap()
            .set_next(variant2_id.clone());

        scene2
            .variants_mut()
            .get_mut(&variant2_id)
            .unwrap()
            .set_next(variant3_id.clone());

        // Add scenes to storyboard
        storyboard.add_scene(scene1);
        storyboard.add_scene(scene2);
        storyboard.add_scene(scene3);

        // Act
        let result: Vec<&Scene> = storyboard.linearize_from(&variant1_id).collect();

        // Assert
        assert_eq!(result.len(), 3);
        assert!(result[0].has_variant(&variant1_id));
        assert!(result[1].has_variant(&variant2_id));
        assert!(result[2].has_variant(&variant3_id));
    }

    #[test]
    fn linearize_from_breaks_when_cycle_detected() {
        // Arrange
        let mut storyboard = Storyboard::default();

        let mut scene1 = Scene::new();
        let mut scene2 = Scene::new();
        let mut scene3 = Scene::new();

        // Get the active variant ID from each scene
        let variant1_id = scene1.active_variant().clone();
        let variant2_id = scene2.active_variant().clone();
        let variant3_id = scene3.active_variant().clone();

        // Wire up next pointers
        scene1
            .variants_mut()
            .get_mut(&variant1_id)
            .unwrap()
            .set_next(variant2_id.clone());

        scene2
            .variants_mut()
            .get_mut(&variant2_id)
            .unwrap()
            .set_next(variant3_id.clone());

        scene3
            .variants_mut()
            .get_mut(&variant3_id)
            .unwrap()
            .set_next(variant1_id.clone());

        // Add scenes to storyboard
        storyboard.add_scene(scene1);
        storyboard.add_scene(scene2);
        storyboard.add_scene(scene3);

        // Act
        let result: Vec<&Scene> = storyboard.linearize_from(&variant1_id).collect();

        // Assert
        assert_eq!(result.len(), 3)
    }

    #[test]
    fn linearize_from_single_scene_returns_one_scene() {
        // Arrange
        let mut storyboard = Storyboard::default();
        let scene = Scene::new();
        let variant_id = scene.active_variant().clone();
        storyboard.add_scene(scene);

        // Act
        let result: Vec<&Scene> = storyboard.linearize_from(&variant_id).collect();

        // Assert
        assert_eq!(result.len(), 1);
        assert!(result[0].has_variant(&variant_id));
    }

    #[test]
    fn linearize_from_missing_variant_returns_empty() {
        // Arrange
        let storyboard = Storyboard::default();
        let phantom_id = Id::<SceneVariant>::new();

        // Act
        let result: Vec<&Scene> = storyboard.linearize_from(&phantom_id).collect();

        // Assert
        assert_eq!(result.len(), 0);
    }
}
