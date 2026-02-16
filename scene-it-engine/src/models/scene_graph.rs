use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

use crate::models::{
    Id,
    scene::Scene,
    storyboard::{StoryboardError, StoryboardUpdate},
};

/// An ordering and relationship model for scenes that expresses what can come next.
///
/// This structure stores only scene relationships (edges and entry points),
/// not scene content. It supports branching paths, optional transitions,
/// and alternate story flows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneGraph {
    edges: HashMap<Id<Scene>, HashSet<Id<Scene>>>,
    roots: HashSet<Id<Scene>>, // Optional story entry points
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
    pub fn add_scene(&mut self, scene_id: &Id<Scene>) -> StoryboardUpdate {
        self.edges.entry(scene_id.clone()).or_default();
        StoryboardUpdate::SceneAdded(scene_id.clone())
    }

    /// Moves a scene from one parent scene to another.
    ///
    /// # Parameters
    /// - `scene`: The scene to move.
    /// - `from`: The current parent scene.
    /// - `dest`: The new parent scene.
    ///
    /// # Errors
    /// Returns `StoryboardError::UnknownScene` if any of the scenes are not present in the graph.
    /// Returns `StoryboardError::InvalidMove` if `scene` is not a child of `from`.
    /// Returns `StoryboardError::CycleDetected` if moving would create a cycle.
    pub fn move_scene(
        &mut self,
        scene: &Id<Scene>,
        from: &Id<Scene>,
        dest: &Id<Scene>,
    ) -> Result<StoryboardUpdate, StoryboardError> {
        // Verify each node exists in the graph
        for s in [scene, from, dest] {
            if !self.edges.contains_key(s) {
                return Err(StoryboardError::SceneNotInGraph(s.clone()));
            }
        }

        // If from and destination node are the same, avoid extra mutation and return
        if from == dest {
            return Ok(StoryboardUpdate::Move {
                scene: scene.clone(),
                from: from.clone(),
                dest: dest.clone(),
            });
        }

        let removed = self
            .edges
            .get_mut(from)
            .expect("Parent existence already checked")
            .remove(scene);

        if !removed {
            return Err(StoryboardError::InvalidMove {
                scene: scene.clone(),
                from: from.clone(),
                dest: dest.clone(),
            });
        }

        if self.is_descendant(&dest, &scene) {
            self.edges
                .get_mut(from)
                .expect("Parent existence already checked")
                .insert(scene.clone());

            return Err(StoryboardError::CycleDetected(scene.clone(), dest.clone()));
        }

        self.edges
            .get_mut(dest)
            .expect("Destination existence already checked")
            .insert(scene.clone());

        Ok(StoryboardUpdate::Move {
            scene: scene.clone(),
            from: from.clone(),
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
    fn is_descendant(&self, start: &Id<Scene>, target: &Id<Scene>) -> bool {
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

    /// Marks a scene as a root (entry point) in the `SceneGraph`.  
    /// The scene is added to the graph if it doesn't already exist.
    pub fn add_root(&mut self, scene_id: &Id<Scene>) -> StoryboardUpdate {
        self.add_scene(scene_id);
        self.roots.insert(scene_id.clone());
        StoryboardUpdate::SceneSetAsRoot(scene_id.clone())
    }

    /// Adds a directed edge from `from` to `to` in the graph, representing a possible next scene.  
    /// If the `to` scene does not exist in the graph, it is added automatically.  
    ///
    /// Example: Scene 3 -> Scene 4 or Scene 3 -> Scene 5
    pub fn add_edge(&mut self, from: &Id<Scene>, dest: &Id<Scene>) -> StoryboardUpdate {
        self.add_scene(from);
        self.add_scene(dest);

        if let Some(node_edges) = self.edges.get_mut(&from) {
            node_edges.insert(dest.clone());
        }

        StoryboardUpdate::LinkedScenes {
            from: from.clone(),
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
    pub fn delete_scene(
        &mut self,
        scene_id: &Id<Scene>,
    ) -> Result<StoryboardUpdate, StoryboardError> {
        // Remove from edges
        if self.edges.remove(&scene_id).is_none() {
            return Err(StoryboardError::SceneNotInGraph(scene_id.clone()));
        }
        // Remove from roots, if needed
        self.roots.remove(scene_id);

        // Remove from scenes connected by edge
        for edges in self.edges.values_mut() {
            edges.remove(scene_id);
        }

        Ok(StoryboardUpdate::SceneDeleted(scene_id.clone()))
    }

    /// Removes a directed edge from one scene to another.
    ///
    /// This operation removes a single possible transition (`from -> to`)
    /// without deleting either scene from the graph. Other outgoing or
    /// incoming edges remain unchanged.
    ///
    /// This is useful for removing optional paths or revising story flow
    /// while keeping both scenes available elsewhere in the graph.
    ///
    /// # Errors
    ///
    /// Returns `StoryboardError::UnknownScene` if the `from` scene does not
    /// exist in the graph.
    pub fn delete_edge(
        &mut self,
        from: &Id<Scene>,
        dest: &Id<Scene>,
    ) -> Result<StoryboardUpdate, StoryboardError> {
        let edges = self
            .edges
            .get_mut(from)
            .ok_or(StoryboardError::SceneNotInGraph(from.clone()))?;

        edges.remove(dest);

        Ok(StoryboardUpdate::EdgeDeleted {
            from: from.clone(),
            dest: dest.clone(),
        })
    }

    /// Returns an iterator over all scenes that are direct successors of `scene_id`.  
    /// These represent all possible "next" scenes in the procedural traversal of the graph.
    pub fn next_scenes(&self, scene_id: &Id<Scene>) -> impl Iterator<Item = &Id<Scene>> {
        self.edges
            .get(scene_id)
            .into_iter()
            .flat_map(|set| set.iter())
    }

    /// Returns all scenes in the graph that cannot be reached from any root.  
    /// These are "orphaned" scenes with no path from a root node, useful for detecting disconnected content.
    pub fn unreachable_scenes(&self) -> HashSet<Id<Scene>> {
        let mut visited_scenes = HashSet::new();
        let mut stack: Vec<_> = self.roots.iter().cloned().collect();

        while let Some(scene) = stack.pop() {
            if visited_scenes.insert(scene.clone()) {
                if let Some(edges) = self.edges.get(&scene) {
                    stack.extend(edges.iter().cloned())
                }
            }
        }

        self.edges
            .keys()
            .cloned()
            .filter(|id| !visited_scenes.contains(id))
            .collect()
    }

    /// Prints the scene graph (or a subtree) using a breadth-first traversal.
    ///
    /// # Parameters
    /// - `from`: Optional starting scene.  
    ///     - `Some(scene_id)` prints the subtree rooted at `scene_id`.  
    ///     - `None` prints the entire graph starting from all roots.
    ///
    /// # Behavior
    /// - Each root is treated as an independent entry point.  
    /// - Scenes reachable from multiple roots are printed only once.  
    /// - Indentation reflects the depth of each scene from the starting point.
    ///
    /// # Examples
    ///
    /// Print the entire graph:
    /// ```
    /// scene_graph.print_from(None);
    /// ```
    ///
    /// Print only the subtree starting at a specific scene:
    /// ```
    /// scene_graph.print_from(Some(scene_id));
    /// ```
    pub fn print_from(&self, from: Option<&Id<Scene>>) {
        if let Some(root) = from {
            return self.print_subtree(root);
        }

        for root in &self.roots {
            println!("ROOT: {root}");
            self.print_subtree(root);
            println!();
        }
    }

    /// Helper method that prints a breadth-first subtree starting at `root`.
    ///
    /// This is a private method used internally by `print_from`.
    ///
    /// - Scenes are printed only once, even if reachable via multiple paths.  
    /// - Each scene is indented according to its distance from `root`.  
    /// - BFS traversal uses a queue and tracks visited scenes.
    fn print_subtree(&self, root: &Id<Scene>) {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back((root, 0));

        while let Some((scene, level)) = queue.pop_front() {
            if !visited.insert(scene) {
                continue;
            }

            println!("{}- {scene}", "  ".repeat(level));

            if let Some(edge) = self.edges.get(scene) {
                for next in edge {
                    queue.push_back((next, level + 1));
                }
            }
        }
    }
}
