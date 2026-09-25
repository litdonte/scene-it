use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::models::{
    HasMetadata, Id, Scene,
    scene_graph::{SceneGraph, SceneGraphError, SceneRemovalDetails},
};

/// Errors that can occur while mutating or querying a [`Narrative`].
#[derive(Debug, Serialize, PartialEq)]
pub enum NarrativeError {
    /// A lower-level [`SceneGraphError`] occurred while updating the scene graph.
    Graph(SceneGraphError),
    /// The referenced scene does not exist in the narrative.
    UnknownScene(Id<Scene>),
    /// No edge connects the two scenes.
    ///
    /// The graph treats removing an absent edge as a no-op. Here it means the
    /// caller asked to remove an edge it believed existed, so its view has
    /// diverged from the engine.
    UnknownEdge { src: Id<Scene>, dest: Id<Scene> },
    /// An edge already connects the two scenes.
    ScenesAlreadyLinked { src: Id<Scene>, dest: Id<Scene> },
    /// A scene with this ID has already been added to the narrative.
    SceneAlreadyExists(Id<Scene>),
    /// The scene is already registered as a root entry point.
    RootAlreadyExists(Id<Scene>),
    /// The scene is not currently registered as a root entry point.
    RootAlreadyRemoved(Id<Scene>),
    /// The scene bank and the scene graph disagree about this scene.
    ///
    /// Unlike the other variants, this does not mean the request was invalid.
    /// It means the narrative's own state is no longer trustworthy, so a
    /// consumer should reload rather than retry.
    InconsistentState(Id<Scene>),
}

impl From<SceneGraphError> for NarrativeError {
    fn from(value: SceneGraphError) -> Self {
        NarrativeError::Graph(value)
    }
}

/// A structural change to a [`Narrative`], emitted as the result of a mutating operation.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum NarrativeUpdate {
    /// A scene was added to the narrative, with no edges and no root status.
    SceneAdded(Id<Scene>),
    /// A scene was removed, along with everything listed in the details.
    SceneRemoved {
        scene: Id<Scene>,
        details: SceneRemovalDetails,
    },
    /// A scene moved from being a child of `src` to being a child of `dest`.
    SceneMoved {
        scene: Id<Scene>,
        src: Id<Scene>,
        dest: Id<Scene>,
    },
    /// A directed link from `src` to `dest` was created.
    ScenesLinked { src: Id<Scene>, dest: Id<Scene> },
    /// A directed link from `src` to `dest` was removed.
    ScenesUnlinked { src: Id<Scene>, dest: Id<Scene> },
    /// A scene was marked as a root entry point.
    SceneSetAsRoot(Id<Scene>),
    /// A scene was unmarked as a root entry point.
    SceneRemovedAsRoot(Id<Scene>),
}

/// The set of scenes and their relationships that make up a story.
///
/// A `Narrative` combines scene data (the `scenes` bank) with a [`SceneGraph`]
/// that tracks ordering, branching, and entry points, keeping the two in sync
/// as scenes are added, removed, linked, and reordered.
///
/// The graph stores only scene IDs and cannot reach scene data, so every
/// mutation here coordinates both: the graph reports whether anything changed,
/// and the narrative translates that into an update and the metadata touches
/// it implies. The narrative is the only producer of [`NarrativeUpdate`].
///
/// A request whose effect already holds is an error here rather than a quiet
/// no-op. Every ID a caller holds originated from this engine, so an ID that
/// does not resolve — or an edge that is not there — means the caller's view
/// has diverged, which is worth surfacing.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Narrative {
    graph: SceneGraph,
    scenes: HashMap<Id<Scene>, Scene>,
}

impl Narrative {
    /// Adds a new scene to the narrative.
    ///
    /// Registers the scene in the scene bank and as a node in the
    /// [`SceneGraph`], with no edges and no root status.
    ///
    /// A new scene has no prior metadata to touch, so nothing is touched here.
    ///
    /// # Errors
    ///
    /// Returns [`NarrativeError::SceneAlreadyExists`] if a scene with this ID
    /// is already in the narrative.
    ///
    /// Returns [`NarrativeError::InconsistentState`] if the graph already knows
    /// a scene the bank does not have.
    pub fn add_scene(&mut self, scene: Scene) -> Result<NarrativeUpdate, NarrativeError> {
        if self.scenes.contains_key(&scene.id()) {
            return Err(NarrativeError::SceneAlreadyExists(scene.id()));
        }

        let scene_id = scene.id();

        if !self.graph.add_scene(scene_id) {
            return Err(NarrativeError::InconsistentState(scene_id));
        }

        self.scenes.insert(scene_id, scene);

        Ok(NarrativeUpdate::SceneAdded(scene_id))
    }

    /// Removes a scene from the narrative and its scene graph.
    ///
    /// Coordinates a deletion across both layers:
    ///
    /// - The scene is removed from the scene bank
    /// - The scene, every edge into or out of it, and its root status are
    ///   removed from the [`SceneGraph`]
    ///
    /// The returned [`SceneRemovalDetails`] carries what came off with it, so a
    /// consumer can drop the node and its connections in one operation.
    ///
    /// # Errors
    ///
    /// Returns [`NarrativeError::UnknownScene`] if the scene is not in the
    /// narrative. Nothing is mutated in that case.
    ///
    /// Returns [`NarrativeError::InconsistentState`] if the scene is in the
    /// bank but not in the graph.
    ///
    /// # Side Effects
    ///
    /// Touches metadata for every scene that lost an edge.
    pub fn remove_scene(&mut self, scene: Id<Scene>) -> Result<NarrativeUpdate, NarrativeError> {
        if !self.scenes.contains_key(&scene) {
            return Err(NarrativeError::UnknownScene(scene));
        }

        let Some(details) = self.graph.remove_scene(scene) else {
            return Err(NarrativeError::InconsistentState(scene));
        };

        let touched: Vec<_> = details
            .edges
            .iter()
            .flat_map(|edge| [edge.src, edge.dest])
            .filter(|id| *id != scene)
            .collect();

        for id in touched {
            self.touch(id);
        }

        self.scenes.remove(&scene);

        Ok(NarrativeUpdate::SceneRemoved { scene, details })
    }

    /// Marks a scene as a root entry point in the scene graph.
    ///
    /// Root scenes are valid starting points for traversal.
    ///
    /// # Errors
    ///
    /// Returns [`NarrativeError::Graph`] with [`SceneGraphError::UnknownScene`]
    /// if the scene is not in the graph.
    ///
    /// Returns [`NarrativeError::RootAlreadyExists`] if the scene is already
    /// registered as a root.
    ///
    /// # Side Effects
    ///
    /// Touches the scene's metadata.
    pub fn set_scene_as_root(
        &mut self,
        scene: Id<Scene>,
    ) -> Result<NarrativeUpdate, NarrativeError> {
        if !self.graph.add_root(scene)? {
            return Err(NarrativeError::RootAlreadyExists(scene));
        }

        self.touch(scene);
        Ok(NarrativeUpdate::SceneSetAsRoot(scene))
    }

    /// Unmarks a scene as a root entry point in the scene graph.
    ///
    /// # Errors
    ///
    /// Returns [`NarrativeError::UnknownScene`] if the scene is not in the
    /// narrative.
    ///
    /// Returns [`NarrativeError::RootAlreadyRemoved`] if the scene is not
    /// currently registered as a root.
    ///
    /// # Side Effects
    ///
    /// Touches the scene's metadata.
    pub fn remove_scene_as_root(
        &mut self,
        scene: Id<Scene>,
    ) -> Result<NarrativeUpdate, NarrativeError> {
        if !self.scenes.contains_key(&scene) {
            return Err(NarrativeError::UnknownScene(scene));
        }

        if !self.graph.remove_root(scene)? {
            return Err(NarrativeError::RootAlreadyRemoved(scene));
        }

        self.touch(scene);

        Ok(NarrativeUpdate::SceneRemovedAsRoot(scene))
    }

    /// Creates a directed link from `src` to `dest`.
    ///
    /// `dest` becomes a possible next scene after `src`. Both scenes must
    /// already exist in the narrative.
    ///
    /// # Errors
    ///
    /// Returns [`NarrativeError::Graph`] with [`SceneGraphError::UnknownScene`]
    /// if either scene is not in the graph.
    ///
    /// Returns [`NarrativeError::ScenesAlreadyLinked`] if the edge already
    /// exists.
    ///
    /// # Side Effects
    ///
    /// Touches metadata for both scenes.
    pub fn link_scenes(
        &mut self,
        src: Id<Scene>,
        dest: Id<Scene>,
    ) -> Result<NarrativeUpdate, NarrativeError> {
        if !self.graph.add_edge(src, dest)? {
            return Err(NarrativeError::ScenesAlreadyLinked { src, dest });
        }

        self.touch(src);
        self.touch(dest);

        Ok(NarrativeUpdate::ScenesLinked { src, dest })
    }

    /// Removes the directed link from `src` to `dest`.
    ///
    /// Disconnects `dest` as a possible next scene after `src`, leaving both
    /// scenes and their other connections in place.
    ///
    /// # Errors
    ///
    /// Returns [`NarrativeError::Graph`] with [`SceneGraphError::UnknownScene`]
    /// if either scene is not in the graph.
    ///
    /// Returns [`NarrativeError::UnknownEdge`] if both exist but no edge
    /// connects them.
    ///
    /// # Side Effects
    ///
    /// Touches metadata for both scenes.
    ///
    /// # Use Cases
    ///
    /// - Removing an optional or branching story path
    /// - Reworking story flow without deleting scenes
    /// - Allowing users to manually prune narrative branches
    pub fn unlink_scenes(
        &mut self,
        src: Id<Scene>,
        dest: Id<Scene>,
    ) -> Result<NarrativeUpdate, NarrativeError> {
        if !self.graph.remove_edge(src, dest)? {
            return Err(NarrativeError::UnknownEdge { src, dest });
        }

        self.touch(src);
        self.touch(dest);

        Ok(NarrativeUpdate::ScenesUnlinked { src, dest })
    }

    /// Moves a scene from one parent to another.
    ///
    /// Changes structure only; no scene content is modified.
    ///
    /// # Errors
    ///
    /// Returns [`NarrativeError::Graph`] wrapping the underlying
    /// [`SceneGraphError`] if any of the three scenes is unknown, if `scene`
    /// is not a child of `src`, or if the move would create a cycle. The graph
    /// is left unchanged on any of those.
    ///
    /// # Side Effects
    ///
    /// Touches metadata for the moved scene and both parents.
    pub fn move_scene(
        &mut self,
        scene: Id<Scene>,
        src: Id<Scene>,
        dest: Id<Scene>,
    ) -> Result<NarrativeUpdate, NarrativeError> {
        self.graph.move_scene(scene, src, dest)?;
        self.touch(scene);
        self.touch(src);
        self.touch(dest);
        Ok(NarrativeUpdate::SceneMoved { scene, src, dest })
    }

    /// Marks a scene as modified.
    ///
    /// Silently does nothing if the scene is not in the bank. Callers that
    /// need a missing scene reported check for it before mutating.
    fn touch(&mut self, scene: Id<Scene>) {
        if let Some(s) = self.scenes.get_mut(&scene) {
            s.touch();
        }
    }
}
