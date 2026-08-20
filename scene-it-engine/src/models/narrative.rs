use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::models::{
    HasMetadata, Id, Scene, SceneVariant,
    scene_graph::{SceneGraph, SceneGraphError, SceneGraphUpdate},
};

/// Errors that can occur while mutating or querying a [`Narrative`].
#[derive(Debug, Serialize, PartialEq)]
pub enum NarrativeError {
    /// A lower-level [`SceneGraphError`] occurred while updating the scene graph.
    Graph(SceneGraphError),
    /// The referenced scene does not exist in the narrative.
    UnknownScene(Id<Scene>),
    /// No edge exists between the given scene variants.
    UnknownEdge {
        src: Id<SceneVariant>,
        dest: Id<SceneVariant>,
    },
    /// The given scene variants are already linked by an edge.
    VariantsAlreadyLinked {
        src: Id<SceneVariant>,
        dest: Id<SceneVariant>,
    },
    /// A scene with this ID has already been added to the narrative.
    SceneAlreadyExists(Id<Scene>),
    /// The scene variant is already registered as a root entry point.
    RootAlreadyExists(Id<SceneVariant>),
    /// The scene variant is already removed as a root entry point.
    RootAlreadyRemoved(Id<SceneVariant>),
}

impl From<SceneGraphError> for NarrativeError {
    fn from(value: SceneGraphError) -> Self {
        NarrativeError::Graph(value)
    }
}

/// A structural change to a [`Narrative`], emitted as the result of a mutating operation.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum NarrativeUpdate {
    /// A change to the underlying scene graph.
    Graph(SceneGraphUpdate),
}

impl From<SceneGraphUpdate> for NarrativeUpdate {
    fn from(value: SceneGraphUpdate) -> Self {
        NarrativeUpdate::Graph(value)
    }
}

/// The set of scenes and their relationships that make up a story.
///
/// A `Narrative` combines scene data (the `scenes` bank) with a [`SceneGraph`]
/// that tracks ordering, branching, and entry points, keeping the two in sync
/// as scenes are added, removed, linked, and reordered.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Narrative {
    graph: SceneGraph,
    scenes: HashMap<Id<Scene>, Scene>,
}

impl Narrative {
    /// Adds a new scene to the narrative.
    ///
    /// Registers the scene in the scene bank and each of its variants in the
    /// [`SceneGraph`], returning one update per variant that was newly added.
    ///
    /// A new scene has no prior metadata to touch, so no updates are applied
    /// to it here.
    ///
    /// # Errors
    ///
    /// Returns [`NarrativeError::SceneAlreadyExists`] if a scene with this ID
    /// is already in the narrative. Overwriting would orphan the previous
    /// scene's variants in the graph with no owner to remove them.
    pub fn add_scene(&mut self, scene: Scene) -> Result<Vec<NarrativeUpdate>, NarrativeError> {
        if self.scenes.contains_key(&scene.id()) {
            return Err(NarrativeError::SceneAlreadyExists(scene.id()));
        }

        // Because the scene is new, updates for added variants won't be recorded in scene metadata.
        let updates: Vec<_> = scene
            .variant_ids()
            .filter_map(|v| self.graph.add_variant(*v))
            .collect();

        self.scenes.insert(scene.id(), scene);

        Ok(updates.into_iter().map(NarrativeUpdate::from).collect())
    }

    /// Removes a scene from the narrative and its scene graph.
    ///
    /// This method performs a coordinated deletion across both ownership layers:
    ///
    /// - The scene is removed from the narrative's `scenes` (data ownership).
    /// - The scene is removed from the `SceneGraph` (structural relationships),
    ///   including:
    ///   - The scene node itself
    ///   - Any edges pointing *to* or *from* the scene
    ///   - Root references, if the scene was an entry point
    ///
    /// # Errors
    ///
    /// Returns [`NarrativeError::UnknownScene`] if the scene does not exist in
    /// the narrative.
    ///
    /// # Side Effects
    ///
    /// - Applies scene graph updates for the removed variants, edges, and roots
    /// - Touches metadata for affected scenes via `apply_scene_graph_update`
    ///
    /// ```
    pub fn remove_scene(
        &mut self,
        scene: Id<Scene>,
    ) -> Result<Vec<NarrativeUpdate>, NarrativeError> {
        if let Some(scene) = self.scenes.remove(&scene) {
            let updates: Vec<_> = scene
                .variant_ids()
                .flat_map(|v| self.graph.remove_variant(*v))
                .collect();

            updates
                .iter()
                .for_each(|u| self.apply_scene_graph_update(u.clone()));

            return Ok(updates.into_iter().map(NarrativeUpdate::from).collect());
        }

        Err(NarrativeError::UnknownScene(scene))
    }

    /// Marks a scene as a root entry point in the scene graph.
    ///
    /// Root scenes represent valid starting points for story traversal.
    ///
    /// # Errors
    ///
    /// Returns [`NarrativeError::RootAlreadyExists`] if the scene variant is
    /// already registered as a root.
    pub fn set_variant_as_root(
        &mut self,
        variant_id: Id<SceneVariant>,
    ) -> Result<NarrativeUpdate, NarrativeError> {
        if let Some(update) = self.graph.add_root(variant_id)? {
            self.apply_scene_graph_update(update.clone());
            return Ok(update.into());
        }

        Err(NarrativeError::RootAlreadyExists(variant_id))
    }

    /// Unmarks a scene as a root entry point in the scene graph.
    ///
    /// # Errors
    ///
    /// Returns [`NarrativeError::RootAlreadyRemoved`] if the scene variant is
    /// not currently registered as a root.
    pub fn remove_variant_as_root(
        &mut self,
        variant_id: Id<SceneVariant>,
    ) -> Result<NarrativeUpdate, NarrativeError> {
        if let Some(update) = self.graph.remove_root(variant_id)? {
            self.apply_scene_graph_update(update.clone());
            return Ok(update.into());
        }

        Err(NarrativeError::RootAlreadyRemoved(variant_id))
    }

    /// Creates a directional link between two scenes.
    ///
    /// The `src` scene variant will be considered a predecessor of the `dest` scene
    /// during traversal and linearization. Both scenes must already exist.
    ///
    /// # Errors
    ///
    /// Returns [`NarrativeError::Graph`] with [`SceneGraphError::UnknownVariant`]
    /// if either `src` or `dest` does not exist in the narrative.
    ///
    /// Returns [`NarrativeError::VariantsAlreadyLinked`] if the edge already exists.
    pub fn link_variants(
        &mut self,
        src: Id<SceneVariant>,
        dest: Id<SceneVariant>,
    ) -> Result<NarrativeUpdate, NarrativeError> {
        let graph_update = self.graph.add_edge(src, dest)?;

        if let Some(update) = graph_update {
            self.apply_scene_graph_update(update.clone());
            return Ok(update.into());
        }

        Err(NarrativeError::VariantsAlreadyLinked { src, dest })
    }

    /// Removes a directed edge between two scene variants.
    ///
    /// Disconnects `dest` as a possible successor of `src` without removing
    /// either variant from the narrative. Other edges into or out of both
    /// variants are unaffected.
    ///
    /// # Errors
    ///
    /// Returns [`NarrativeError::Graph`] with [`SceneGraphError::UnknownVariant`]
    /// if either `src` or `dest` is not in the graph.
    ///
    /// Returns [`NarrativeError::UnknownEdge`] if both exist but no edge
    /// connects them. The graph treats this as a no-op; here it means the
    /// caller asked to remove an edge it believed existed, so its view has
    /// diverged from the engine.
    ///
    /// # Side Effects
    ///
    /// Touches metadata for both variants' scenes.
    ///
    /// # Use Cases
    ///
    /// - Removing an optional or branching story path
    /// - Reworking story flow without deleting scenes
    /// - Allowing users to manually prune narrative branches
    pub fn unlink_variants(
        &mut self,
        src: Id<SceneVariant>,
        dest: Id<SceneVariant>,
    ) -> Result<NarrativeUpdate, NarrativeError> {
        if let Some(graph_update) = self.graph.remove_edge(src, dest)? {
            self.apply_scene_graph_update(graph_update.clone());
            return Ok(graph_update.into());
        }

        Err(NarrativeError::UnknownEdge { src, dest })
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
        self.graph.unreachable_variants();
        todo!()
    }

    /// Moves a scene variant from one parent variant to another.
    ///
    /// Changes structure only; no scene content is modified.
    ///
    /// # Errors
    ///
    /// Returns [`NarrativeError::Graph`] wrapping the underlying
    /// [`SceneGraphError`] if any of the three variants is unknown, if
    /// `variant` is not a child of `src`, or if the move would create a cycle.
    /// On failure the graph is left unchanged.
    ///
    /// # Side Effects
    ///
    /// Touches metadata for the moved variant's scene and both parents'.
    pub fn move_variant(
        &mut self,
        variant: Id<SceneVariant>,
        src: Id<SceneVariant>,
        dest: Id<SceneVariant>,
    ) -> Result<NarrativeUpdate, NarrativeError> {
        let graph_update = self.graph.move_variant(variant, src, dest)?;
        self.apply_scene_graph_update(graph_update.clone());
        Ok(graph_update.into())
    }

    /// Returns an iterator over the scenes reachable from `root`, in traversal order.
    ///
    /// Starting at `root`, this follows each scene variant's `next` link until it
    /// reaches a variant with no successor or revisits an already-visited variant
    /// (at which point traversal stops to avoid looping). If a variant referenced
    /// by the graph cannot be found in the scene bank, a warning is printed to
    /// stderr and traversal stops.
    pub fn linearize_from<'a>(&'a self, root: Id<SceneVariant>) -> impl Iterator<Item = &'a Scene> {
        let mut current = Some(root);
        let mut visited = HashSet::new();
        let mut order = Vec::new();

        while let Some(variant_id) = current {
            if !visited.insert(variant_id) {
                break;
            }

            if let Some(scene) = self.scenes.values().find(|s| s.has_variant(&variant_id)) {
                order.push(scene);

                current = scene
                    .variants()
                    .get(&variant_id)
                    .and_then(|v| v.next())
                    .copied();
            } else {
                eprintln!("Warning: variant ID {variant_id} found in graph but not in any scene");
            }
        }

        order.into_iter()
    }

    /// Applies a structural update emitted by the scene graph.
    ///
    /// The graph holds only variant IDs and cannot reach scene data, so it
    /// reports what changed and the narrative translates that into metadata
    /// effects on the scenes involved. This is the only reason the update
    /// types cross the boundary internally; forwarding them to the UI is
    /// separate.
    fn apply_scene_graph_update(&mut self, update: SceneGraphUpdate) {
        match update {
            SceneGraphUpdate::Move { variant, src, dest } => {
                self.update_metadata(variant);
                self.update_metadata(src);
                self.update_metadata(dest);
            }
            SceneGraphUpdate::SceneVariantAdded(variant)
            | SceneGraphUpdate::SceneVariantRemoved(variant)
            | SceneGraphUpdate::SceneVariantSetAsRoot(variant)
            | SceneGraphUpdate::SceneVariantRemovedAsRoot(variant) => {
                self.update_metadata(variant);
            }
            SceneGraphUpdate::EdgeAdded { src, dest }
            | SceneGraphUpdate::EdgeRemoved { src, dest } => {
                self.update_metadata(src);
                self.update_metadata(dest);
            }
        }
    }

    /// Updates metadata for a scene by marking it as modified.
    ///
    /// This is typically called after structural changes such as moves,
    /// edge updates, or deletions.
    fn update_metadata(&mut self, variant_id: Id<SceneVariant>) {
        for scene in self.scenes.values_mut() {
            if scene.has_variant(&variant_id) {
                scene.touch();
            }
        }
    }
}
