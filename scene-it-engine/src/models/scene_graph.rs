use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, hash_map::Entry};

use crate::models::{Id, scene::SceneVariant};

/// A structural change to a [`SceneGraph`], emitted as the result of a mutating operation.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum SceneGraphUpdate {
    /// `variant` was moved from being a child of `src` to being a child of `dest`.
    Move {
        variant: Id<SceneVariant>,
        src: Id<SceneVariant>,
        dest: Id<SceneVariant>,
    },
    /// A scene variant was added to the graph.
    SceneVariantAdded(Id<SceneVariant>),
    /// A scene variant was removed from the graph.
    SceneVariantRemoved(Id<SceneVariant>),
    /// A scene variant was marked as a root entry point.
    SceneVariantSetAsRoot(Id<SceneVariant>),
    /// A scene variant was unmarked as a root entry point.
    SceneVariantRemovedAsRoot(Id<SceneVariant>),
    /// A directed edge from `src` to `dest` was added.
    EdgeAdded {
        src: Id<SceneVariant>,
        dest: Id<SceneVariant>,
    },
    /// A directed edge from `src` to `dest` was removed.
    EdgeRemoved {
        src: Id<SceneVariant>,
        dest: Id<SceneVariant>,
    },
}

/// Errors that can occur while mutating or querying a [`SceneGraph`].
#[derive(Debug, Serialize, PartialEq)]
pub enum SceneGraphError {
    /// The referenced scene variant does not exist in the graph.
    UnknownVariant(Id<SceneVariant>),
    /// `variant` could not be moved because it is not a child of `src`.
    InvalidMove {
        variant: Id<SceneVariant>,
        src: Id<SceneVariant>,
        dest: Id<SceneVariant>,
    },
    /// Moving `variant` under `dest` would create a cycle in the graph.
    CycleDetected {
        variant: Id<SceneVariant>,
        dest: Id<SceneVariant>,
    },
}

/// An ordering and relationship model for scenes that expresses what can come next.
///
/// This structure stores only scene relationships (edges and entry points),
/// not scene content. It supports branching paths, optional transitions,
/// and alternate story flows.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct SceneGraph {
    /// Adjacency list mapping each scene variant to its direct successors.
    edges: HashMap<Id<SceneVariant>, HashSet<Id<SceneVariant>>>,
    /// Scene variants that are valid starting points for traversal.
    roots: HashSet<Id<SceneVariant>>, // Optional story entry points
}

impl SceneGraph {
    /// Creates an empty `SceneGraph` with no variants, edges, or roots.
    pub fn new() -> Self {
        Self {
            edges: HashMap::new(),
            roots: HashSet::new(),
        }
    }

    /// Adds a scene to the `SceneGraph`.  
    /// If the scene does not exist, it is initialized with an empty set of edges.
    pub fn add_variant(&mut self, variant_id: &Id<SceneVariant>) -> Option<SceneGraphUpdate> {
        match self.edges.entry(variant_id.clone()) {
            Entry::Vacant(entry) => {
                entry.insert(HashSet::new());
                Some(SceneGraphUpdate::SceneVariantAdded(variant_id.clone()))
            }
            Entry::Occupied(_) => None,
        }
    }

    /// Removes a scene from the `SceneGraph`.
    ///
    /// This operation:
    /// - Removes the scene itself from the graph.
    /// - Removes the scene from the set of root entry points, if present.
    /// - Removes all incoming edges that reference this scene from other scenes.
    ///
    /// After this call, the scene will no longer participate in traversal,
    /// linearization, or reachability analysis.
    pub fn remove_variant(&mut self, variant_id: &Id<SceneVariant>) -> Vec<SceneGraphUpdate> {
        let mut variant_deletion_updates = Vec::new();

        // Outgoing
        let outgoing_edges: Vec<_> = self
            .edges
            .get(variant_id)
            .map(|dests| dests.iter().cloned().collect())
            .unwrap_or_default();

        // Incoming
        let incoming_edges: Vec<_> = self
            .edges
            .iter()
            .filter(|(_, dests)| dests.contains(variant_id))
            .map(|(src, _)| src.clone())
            .collect();

        // Delete edges
        variant_deletion_updates.extend(
            outgoing_edges
                .iter()
                .filter_map(|dest| self.remove_edge_unchecked(variant_id, dest)),
        );

        variant_deletion_updates.extend(
            incoming_edges
                .iter()
                .filter_map(|src| self.remove_edge_unchecked(src, variant_id)),
        );

        // Remove from roots, if exists
        if self.roots.remove(variant_id) {
            variant_deletion_updates.push(SceneGraphUpdate::SceneVariantRemovedAsRoot(
                variant_id.clone(),
            ))
        }

        // Remove from edges
        if self.edges.remove(variant_id).is_some() {
            variant_deletion_updates
                .push(SceneGraphUpdate::SceneVariantRemoved(variant_id.clone()));
        }

        variant_deletion_updates
    }

    /// Moves a scene variant from one parent scene variant to another.
    ///
    /// # Parameters
    /// - `variant`: The scene variant to move.
    /// - `src`: The current parent scene variant.
    /// - `dest`: The new parent scene variant.
    ///
    /// # Errors
    /// Returns `SceneGraphError::UnknownVariant` if any of the scenes variants are not present in the graph.
    /// Returns `SceneGraphError::InvalidMove` if `variant` is not a child of `from`.
    /// Returns `SceneGraphError::CycleDetected` if moving would create a cycle.
    pub fn move_variant(
        &mut self,
        variant: &Id<SceneVariant>,
        src: &Id<SceneVariant>,
        dest: &Id<SceneVariant>,
    ) -> Result<SceneGraphUpdate, SceneGraphError> {
        // Verify each node exists in the graph
        for s in [variant, src, dest] {
            if !self.edges.contains_key(s) {
                return Err(SceneGraphError::UnknownVariant(s.clone()));
            }
        }

        if !self
            .edges
            .get_mut(src)
            .is_some_and(|edges| edges.remove(variant))
        {
            return Err(SceneGraphError::InvalidMove {
                variant: variant.clone(),
                src: src.clone(),
                dest: dest.clone(),
            });
        }

        if self.is_descendant(variant, dest) {
            // Getting the edges for the source should always return as Some
            if let Some(edges) = self.edges.get_mut(src) {
                edges.insert(variant.clone());
            }

            return Err(SceneGraphError::CycleDetected {
                variant: variant.clone(),
                dest: dest.clone(),
            });
        }

        if let Some(edges) = self.edges.get_mut(dest) {
            edges.insert(variant.clone());
        }

        Ok(SceneGraphUpdate::Move {
            variant: variant.clone(),
            src: src.clone(),
            dest: dest.clone(),
        })
    }

    /// Marks a scene variant as a root (entry point) in the `SceneGraph`.  
    /// The scene variant is added to the graph if it doesn't already exist.
    pub fn add_root(
        &mut self,
        variant_id: &Id<SceneVariant>,
    ) -> Result<Option<SceneGraphUpdate>, SceneGraphError> {
        if !self.edges.contains_key(variant_id) {
            return Err(SceneGraphError::UnknownVariant(variant_id.clone()));
        }

        if self.roots.insert(variant_id.clone()) {
            return Ok(Some(SceneGraphUpdate::SceneVariantSetAsRoot(
                variant_id.clone(),
            )));
        }

        Ok(None)
    }

    /// Unmarks a scene variant as a root (entry point) in the `SceneGraph`.
    ///
    /// Returns `None` if the variant was not registered as a root.
    pub fn remove_root(&mut self, variant_id: &Id<SceneVariant>) -> Option<SceneGraphUpdate> {
        if self.roots.remove(variant_id) {
            return Some(SceneGraphUpdate::SceneVariantRemovedAsRoot(
                variant_id.clone(),
            ));
        }

        None
    }

    /// Adds a directed edge from `src` to `dest` in the graph, representing a possible next scene.  
    /// If the `to` scene does not exist in the graph, it is added automatically.  
    ///
    /// Example: Scene 3 -> Scene 4 or Scene 3 -> Scene 5
    pub fn add_edge(
        &mut self,
        src: &Id<SceneVariant>,
        dest: &Id<SceneVariant>,
    ) -> Result<Option<SceneGraphUpdate>, SceneGraphError> {
        if !self.edges.contains_key(src) {
            return Err(SceneGraphError::UnknownVariant(src.clone()));
        }

        if !self.edges.contains_key(dest) {
            return Err(SceneGraphError::UnknownVariant(dest.clone()));
        }

        if self
            .edges
            .get_mut(src)
            .is_some_and(|e| e.insert(dest.clone()))
        {
            return Ok(Some(SceneGraphUpdate::EdgeAdded {
                src: src.clone(),
                dest: dest.clone(),
            }));
        }

        Ok(None)
    }

    /// Removes a directed edge from one scene variant to another.
    ///
    /// This operation removes a single possible transition (`src -> dest`)
    /// without deleting either scene variant from the graph. Other outgoing or
    /// incoming edges remain unchanged.
    ///
    /// This is useful for removing optional paths or revising story flow
    /// while keeping both scenes available elsewhere in the graph.
    ///
    /// # Errors
    ///
    /// Returns `SceneGraphError::UnknownScene` if the `src` scene does not
    /// exist in the graph.
    pub fn remove_edge(
        &mut self,
        src: &Id<SceneVariant>,
        dest: &Id<SceneVariant>,
    ) -> Result<Option<SceneGraphUpdate>, SceneGraphError> {
        if !self.edges.contains_key(src) {
            return Err(SceneGraphError::UnknownVariant(src.clone()));
        }

        if !self.edges.contains_key(dest) {
            return Err(SceneGraphError::UnknownVariant(dest.clone()));
        }

        Ok(self.remove_edge_unchecked(src, dest))
    }

    /// Removes the edge from `src` to `dest` without validating that either
    /// scene variant exists in the graph.
    fn remove_edge_unchecked(
        &mut self,
        src: &Id<SceneVariant>,
        dest: &Id<SceneVariant>,
    ) -> Option<SceneGraphUpdate> {
        if self.edges.get_mut(src).is_some_and(|e| e.remove(dest)) {
            return Some(SceneGraphUpdate::EdgeRemoved {
                src: src.clone(),
                dest: dest.clone(),
            });
        }

        None
    }

    /// Returns an iterator over all scenes that are direct successors of `scene_id`.  
    /// These represent all possible "next" scenes in the procedural traversal of the graph.
    pub fn next_variants(
        &self,
        variant_id: &Id<SceneVariant>,
    ) -> impl Iterator<Item = &Id<SceneVariant>> {
        self.edges
            .get(variant_id)
            .into_iter()
            .flat_map(|set| set.iter())
    }

    /// Returns all scenes in the graph that cannot be reached from any root.  
    /// These are "orphaned" scenes with no path from a root node, useful for detecting disconnected content.
    pub fn unreachable_variants(&self) -> HashSet<Id<SceneVariant>> {
        let mut visited = HashSet::new();
        let mut stack: Vec<_> = self.roots.iter().cloned().collect();

        while let Some(variant) = stack.pop() {
            if visited.insert(variant.clone()) {
                if let Some(edges) = self.edges.get(&variant) {
                    stack.extend(edges.iter().cloned())
                }
            }
        }

        self.edges
            .keys()
            .cloned()
            .filter(|id| !visited.contains(id))
            .collect()
    }

    /// Returns an iterator over all scene variants reachable from `root`, in
    /// depth-first traversal order (including `root` itself).
    pub fn reachable_from<'a>(
        &'a self,
        root: &'a Id<SceneVariant>,
    ) -> impl Iterator<Item = &'a Id<SceneVariant>> {
        let mut visited = HashSet::new();
        let mut order = Vec::new();
        let mut stack = vec![root];

        while let Some(current) = stack.pop() {
            if visited.insert(current) {
                order.push(current);
                if let Some(children) = self.edges.get(current) {
                    stack.extend(children);
                }
            }
        }

        order.into_iter()
    }

    /// Determines whether `target` is reachable from `start` in the scene graph.
    ///
    /// This method performs a depth-first traversal beginning at `start` and
    /// follows outgoing edges to check if `target` appears anywhere downstream.
    /// It is commonly used to:
    ///
    /// - Prevent cycles when adding or moving edges
    /// - Validate scene reordering operations
    /// - Reason about ancestor/descendant relationships between scenes
    ///
    /// # Parameters
    /// - `start`: The scene from which traversal begins.
    /// - `target`: The scene being checked for reachability.
    ///
    /// # Returns
    /// - `true` if `target` is a descendant of `start`
    /// - `false` if no path exists from `start` to `target`
    ///
    /// # Notes
    /// - The traversal short-circuits as soon as `target` is found.
    /// - Visited scenes are tracked to avoid infinite loops in cyclic graphs.
    /// - This method does not mutate the graph.
    fn is_descendant(&self, start: &Id<SceneVariant>, target: &Id<SceneVariant>) -> bool {
        let mut visited = HashSet::new();
        let mut stack = vec![start];

        while let Some(node) = stack.pop() {
            if node == target {
                return true;
            }

            if visited.insert(node) {
                if let Some(edges) = self.edges.get(node) {
                    stack.extend(edges);
                }
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {}
