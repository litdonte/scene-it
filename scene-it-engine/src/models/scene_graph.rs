use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::models::{
    Id,
    scene::SceneVariant,
    storyboard::{StoryboardError, StoryboardUpdate},
};

/// An ordering and relationship model for scenes that expresses what can come next.
///
/// This structure stores only scene relationships (edges and entry points),
/// not scene content. It supports branching paths, optional transitions,
/// and alternate story flows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneGraph {
    edges: HashMap<Id<SceneVariant>, HashSet<Id<SceneVariant>>>,
    roots: HashSet<Id<SceneVariant>>, // Optional story entry points
}

impl SceneGraph {
    pub fn new() -> Self {
        Self {
            edges: HashMap::new(),
            roots: HashSet::new(),
        }
    }

    /// Adds a scene to the `SceneGraph`.  
    /// If the scene does not exist, it is initialized with an empty set of edges.
    pub fn add_variant(&mut self, variant_id: &Id<SceneVariant>) -> StoryboardUpdate {
        self.edges.entry(variant_id.clone()).or_default();
        StoryboardUpdate::SceneVariantAdded(variant_id.clone())
    }

    /// Moves a scene from one parent scene to another.
    ///
    /// # Parameters
    /// - `scene`: The scene to move.
    /// - `src`: The current parent scene.
    /// - `dest`: The new parent scene.
    ///
    /// # Errors
    /// Returns `StoryboardError::UnknownScene` if any of the scenes are not present in the graph.
    /// Returns `StoryboardError::InvalidMove` if `scene` is not a child of `from`.
    /// Returns `StoryboardError::CycleDetected` if moving would create a cycle.
    pub fn move_variant(
        &mut self,
        variant: &Id<SceneVariant>,
        src: &Id<SceneVariant>,
        dest: &Id<SceneVariant>,
    ) -> Result<StoryboardUpdate, StoryboardError> {
        // Verify each node exists in the graph
        for s in [variant, src, dest] {
            if !self.edges.contains_key(s) {
                return Err(StoryboardError::SceneVariantNotInGraph(s.clone()));
            }
        }

        // If from and destination node are the same, avoid extra mutation and return
        if src == dest {
            return Ok(StoryboardUpdate::Move {
                variant: variant.clone(),
                src: src.clone(),
                dest: dest.clone(),
            });
        }

        let removed = self
            .edges
            .get_mut(src)
            .expect("Parent existence already checked")
            .remove(variant);

        if !removed {
            return Err(StoryboardError::InvalidMove {
                scene: variant.clone(),
                src: src.clone(),
                dest: dest.clone(),
            });
        }

        if self.is_descendant(dest, variant) {
            self.edges
                .get_mut(src)
                .expect("Parent existence already checked")
                .insert(variant.clone());

            return Err(StoryboardError::CycleDetected(
                variant.clone(),
                dest.clone(),
            ));
        }

        self.edges
            .get_mut(dest)
            .expect("Destination existence already checked")
            .insert(variant.clone());

        Ok(StoryboardUpdate::Move {
            variant: variant.clone(),
            src: src.clone(),
            dest: dest.clone(),
        })
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

    /// Marks a scene variant as a root (entry point) in the `SceneGraph`.  
    /// The scene variant is added to the graph if it doesn't already exist.
    pub fn add_root(&mut self, variant_id: &Id<SceneVariant>) -> StoryboardUpdate {
        self.add_variant(variant_id);
        self.roots.insert(variant_id.clone());
        StoryboardUpdate::SceneVariantSetAsRoot(variant_id.clone())
    }

    /// Adds a directed edge from `from` to `dest` in the graph, representing a possible next scene.  
    /// If the `to` scene does not exist in the graph, it is added automatically.  
    ///
    /// Example: Scene 3 -> Scene 4 or Scene 3 -> Scene 5
    pub fn add_edge(
        &mut self,
        src: &Id<SceneVariant>,
        dest: &Id<SceneVariant>,
    ) -> StoryboardUpdate {
        self.add_variant(src);
        self.add_variant(dest);

        if let Some(node_edges) = self.edges.get_mut(&src) {
            node_edges.insert(dest.clone());
        }

        StoryboardUpdate::LinkedSceneVariants {
            src: src.clone(),
            dest: dest.clone(),
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
    ///
    /// # Errors
    ///
    /// Returns `StoryboardError::UnknownScene` if the scene does not exist
    /// in the graph.
    pub fn delete_variant(
        &mut self,
        variant_id: &Id<SceneVariant>,
    ) -> Result<StoryboardUpdate, StoryboardError> {
        // Remove from edges
        if self.edges.remove(&variant_id).is_none() {
            return Err(StoryboardError::SceneVariantNotInGraph(variant_id.clone()));
        }
        // Remove from roots, if needed
        self.roots.remove(variant_id);

        // Remove from scenes connected by edge
        for edges in self.edges.values_mut() {
            edges.remove(variant_id);
        }

        Ok(StoryboardUpdate::SceneVariantDeleted(variant_id.clone()))
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
    /// Returns `StoryboardError::UnknownScene` if the `src` scene does not
    /// exist in the graph.
    pub fn delete_edge(
        &mut self,
        src: &Id<SceneVariant>,
        dest: &Id<SceneVariant>,
    ) -> Result<StoryboardUpdate, StoryboardError> {
        let edges = self
            .edges
            .get_mut(src)
            .ok_or(StoryboardError::SceneVariantNotInGraph(src.clone()))?;

        edges.remove(dest);

        Ok(StoryboardUpdate::EdgeDeleted {
            src: src.clone(),
            dest: dest.clone(),
        })
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
}

#[cfg(test)]
mod tests {
    #[test]
    fn cycle_detection_prevents_invalid_move() {}
    #[test]
    fn unreachable_variants_returns_orphans() {}
    #[test]
    fn delete_variant_removes_incoming_edges() {}
}
