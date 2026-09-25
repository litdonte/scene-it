use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, hash_map::Entry};

use crate::models::{Id, scene::Scene};

/// Errors that can occur while mutating or querying a [`SceneGraph`].
#[derive(Debug, Serialize, PartialEq)]
pub enum SceneGraphError {
    /// The referenced scene does not exist in the graph.
    UnknownScene(Id<Scene>),
    /// `scene` could not be moved because it is not a child of `src`.
    InvalidMove {
        scene: Id<Scene>,
        src: Id<Scene>,
        dest: Id<Scene>,
    },
    /// Moving `scene` under `dest` would create a cycle in the graph.
    CycleDetected { scene: Id<Scene>, dest: Id<Scene> },
}

/// A directed connection from one scene to another.
#[derive(Debug, PartialEq, Serialize, Clone)]
pub struct Edge {
    pub src: Id<Scene>,
    pub dest: Id<Scene>,
}

/// What came off the graph along with a removed scene.
///
/// Returned by [`SceneGraph::remove_scene`] so a caller can drop the node and
/// everything attached to it in one operation, rather than querying for the
/// scene's connections before removing it.
#[derive(Debug, PartialEq, Serialize, Clone)]
pub struct SceneRemovalDetails {
    /// Every edge into or out of the removed scene, outgoing before incoming.
    pub edges: Vec<Edge>,
    /// Whether the removed scene was a root entry point.
    pub was_root: bool,
}

/// An ordering and relationship model for scenes that expresses what can come next.
///
/// This structure stores only scene relationships (edges and entry points),
/// not scene content. It supports branching paths, optional transitions,
/// and alternate story flows.
///
/// # What mutations report
///
/// The graph reports facts, not events. Most mutations return a `bool` saying
/// whether anything changed; [`SceneGraph::remove_scene`] returns details
/// because it touches state the caller did not name, and those neighbors
/// cannot be predicted from the arguments alone.
///
/// [`Narrative`] owns the update vocabulary and is its only producer. Keeping
/// it there means the graph has no knowledge of its consumer, and the same
/// update type can describe changes the graph knows nothing about.
///
/// # bool versus Result
///
/// The two outcomes are distinct and both are represented:
///
/// - `Ok(false)` means the request was valid and its effect already held.
///   Adding an existing scene, linking an already-linked pair, removing an
///   absent edge: nothing to do, nothing to report.
/// - `Err(SceneGraphError)` means the request could not be honored. An
///   unrecognized id, a move that would create a cycle, a scene that is not a
///   child of the parent it is being moved from.
///
/// Nothing is created implicitly to make a request succeed. An id the graph
/// does not recognize is an error rather than a node to materialize, since
/// every id a caller holds originated here.
///
/// [`Narrative`] layers a stricter contract on top: several of these no-ops
/// become errors there, because a frontend asking to unlink an edge it can see
/// has a view that has diverged from the engine. That distinction belongs at
/// the boundary where requests arrive from outside. Inside the graph, no-ops
/// stay quiet — [`SceneGraph::remove_scene`] calls
/// [`SceneGraph::remove_edge_unchecked`] in loops where absence is expected,
/// and a self-loop legitimately visits the same pair twice.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SceneGraph {
    /// Adjacency list mapping each scene to its direct successors.
    nodes: HashMap<Id<Scene>, HashSet<Id<Scene>>>,
    /// Scenes that are valid starting points for traversal.
    roots: HashSet<Id<Scene>>,
}

impl SceneGraph {
    /// Creates an empty `SceneGraph` with no scenes, edges, or roots.
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            roots: HashSet::new(),
        }
    }

    /// Adds a scene to the `SceneGraph`, initialized with no outgoing edges.
    ///
    /// Returns `false` if the scene is already in the graph, since nothing
    /// changed.
    pub fn add_scene(&mut self, scene_id: Id<Scene>) -> bool {
        match self.nodes.entry(scene_id) {
            Entry::Vacant(entry) => {
                entry.insert(HashSet::new());
                true
            }
            Entry::Occupied(_) => false,
        }
    }

    /// Removes a scene from the `SceneGraph`.
    ///
    /// This operation:
    /// - Removes every edge into or out of the scene
    /// - Removes the scene from the set of root entry points, if present
    /// - Removes the scene itself
    ///
    /// Returns [`SceneRemovalDetails`] describing what came off with it, so a
    /// caller patching a rendered graph can drop the node and its connections
    /// together.
    ///
    /// Returns `None` if the scene is not in the graph. A scene that was
    /// present but isolated returns `Some` with an empty edge list.
    pub fn remove_scene(&mut self, scene_id: Id<Scene>) -> Option<SceneRemovalDetails> {
        let mut deleted_edges = Vec::new();
        let mut removal_details = SceneRemovalDetails {
            edges: vec![],
            was_root: false,
        };

        // Outgoing
        let outgoing_edges: Vec<_> = self
            .nodes
            .get(&scene_id)
            .map(|dests| dests.iter().cloned().collect())
            .unwrap_or_default();

        // Incoming
        let incoming_edges: Vec<_> = self
            .nodes
            .iter()
            .filter(|(_, dests)| dests.contains(&scene_id))
            .map(|(src, _)| *src)
            .collect();

        // Delete edges
        deleted_edges.extend(
            outgoing_edges
                .iter()
                .filter_map(|dest| self.remove_edge_unchecked(scene_id, *dest)),
        );

        deleted_edges.extend(
            incoming_edges
                .iter()
                .filter_map(|src| self.remove_edge_unchecked(*src, scene_id)),
        );

        removal_details.edges = deleted_edges;

        // Remove from roots, if exists
        if self.roots.remove(&scene_id) {
            removal_details.was_root = true;
        }

        // Remove the node itself
        if self.nodes.remove(&scene_id).is_some() {
            return Some(removal_details);
        }

        None
    }

    /// Moves a scene from one parent to another.
    ///
    /// # Parameters
    /// - `scene`: The scene to move.
    /// - `src`: The current parent.
    /// - `dest`: The new parent.
    ///
    /// # Errors
    /// Returns `SceneGraphError::UnknownScene` if `scene`, `src`, or `dest`
    /// is not present in the graph.
    /// Returns `SceneGraphError::InvalidMove` if `scene` is not a child of `src`.
    /// Returns `SceneGraphError::CycleDetected` if the move would create a cycle.
    /// On any failure the graph is left unchanged.
    pub fn move_scene(
        &mut self,
        scene: Id<Scene>,
        src: Id<Scene>,
        dest: Id<Scene>,
    ) -> Result<(), SceneGraphError> {
        // Verify each node exists in the graph
        for s in [scene, src, dest] {
            if !self.nodes.contains_key(&s) {
                return Err(SceneGraphError::UnknownScene(s));
            }
        }

        if !self
            .nodes
            .get_mut(&src)
            .is_some_and(|edges| edges.remove(&scene))
        {
            return Err(SceneGraphError::InvalidMove { scene, src, dest });
        }

        // Argument order matters. This asks whether dest is reachable from scene.
        // The move adds dest -> scene, so a forward path from scene back to dest
        // would close a loop. The reverse question (is scene reachable from dest)
        // only detects a redundant path, which is legal.
        if self.is_descendant(scene, dest) {
            // Getting the edges for the source should always return as Some
            if let Some(edges) = self.nodes.get_mut(&src) {
                edges.insert(scene);
            }

            return Err(SceneGraphError::CycleDetected { scene, dest });
        }

        if let Some(edges) = self.nodes.get_mut(&dest) {
            edges.insert(scene);
        }

        Ok(())
    }

    /// Marks a scene as a root (entry point) in the `SceneGraph`.
    ///
    /// Returns `Ok(false)` if the scene is already a root, since nothing
    /// changed.
    ///
    /// # Errors
    ///
    /// Returns `SceneGraphError::UnknownScene` if the scene is not in the
    /// graph. Roots are never created implicitly: an unrecognized id means the
    /// caller's view of the graph has diverged, which is worth surfacing.
    pub fn add_root(&mut self, scene_id: Id<Scene>) -> Result<bool, SceneGraphError> {
        if !self.nodes.contains_key(&scene_id) {
            return Err(SceneGraphError::UnknownScene(scene_id));
        }

        if self.roots.insert(scene_id) {
            return Ok(true);
        }

        Ok(false)
    }

    /// Unmarks a scene as a root (entry point) in the `SceneGraph`.
    ///
    /// Returns `Ok(false)` if the scene was not registered as a root, since
    /// nothing changed.
    ///
    /// # Errors
    ///
    /// Returns `SceneGraphError::UnknownScene` if the scene is not in the
    /// graph.
    pub fn remove_root(&mut self, scene_id: Id<Scene>) -> Result<bool, SceneGraphError> {
        if !self.nodes.contains_key(&scene_id) {
            return Err(SceneGraphError::UnknownScene(scene_id));
        }

        if self.roots.remove(&scene_id) {
            return Ok(true);
        }

        Ok(false)
    }

    /// Adds a directed edge from `src` to `dest`, representing a possible next
    /// scene.
    ///
    /// Returns `Ok(false)` if the edge already exists, since nothing changed.
    ///
    /// Example: Scene 3 -> Scene 4 or Scene 3 -> Scene 5
    ///
    /// # Errors
    ///
    /// Returns `SceneGraphError::UnknownScene` if either `src` or `dest` is
    /// not in the graph. Neither is created implicitly.
    pub fn add_edge(&mut self, src: Id<Scene>, dest: Id<Scene>) -> Result<bool, SceneGraphError> {
        if !self.nodes.contains_key(&src) {
            return Err(SceneGraphError::UnknownScene(src));
        }

        if !self.nodes.contains_key(&dest) {
            return Err(SceneGraphError::UnknownScene(dest));
        }

        if self.nodes.get_mut(&src).is_some_and(|e| e.insert(dest)) {
            return Ok(true);
        }

        Ok(false)
    }

    /// Removes a directed edge from one scene to another.
    ///
    /// This operation removes a single possible transition (`src -> dest`)
    /// without deleting either scene from the graph. Other outgoing or
    /// incoming edges remain unchanged.
    ///
    /// This is useful for removing optional paths or revising story flow
    /// while keeping both scenes available elsewhere in the graph.
    ///
    /// # Errors
    ///
    /// Returns `SceneGraphError::UnknownScene` if either `src` or `dest` is
    /// not in the graph. Returns `Ok(false)` if both exist but no edge
    /// connects them, since nothing changed.
    pub fn remove_edge(
        &mut self,
        src: Id<Scene>,
        dest: Id<Scene>,
    ) -> Result<bool, SceneGraphError> {
        if !self.nodes.contains_key(&src) {
            return Err(SceneGraphError::UnknownScene(src));
        }

        if !self.nodes.contains_key(&dest) {
            return Err(SceneGraphError::UnknownScene(dest));
        }

        if self.remove_edge_unchecked(src, dest).is_some() {
            return Ok(true);
        }

        Ok(false)
    }

    /// Removes the edge from `src` to `dest` without validating that either
    /// scene exists in the graph.
    ///
    /// Called from [`SceneGraph::remove_scene`], where the ids come from the
    /// graph's own adjacency data and absence is expected — a self-loop visits
    /// the same pair twice, and the second call legitimately finds nothing.
    fn remove_edge_unchecked(&mut self, src: Id<Scene>, dest: Id<Scene>) -> Option<Edge> {
        if self.nodes.get_mut(&src).is_some_and(|e| e.remove(&dest)) {
            return Some(Edge { src, dest });
        }

        None
    }

    /// Returns an iterator over all scenes that are direct successors of
    /// `scene_id` — every scene that could come next.
    pub fn next_scenes(&self, scene_id: Id<Scene>) -> impl Iterator<Item = Id<Scene>> {
        self.nodes
            .get(&scene_id)
            .into_iter()
            .flat_map(|set| set.iter().cloned())
    }

    /// Returns all scenes in the graph that cannot be reached from any root.
    ///
    /// These are orphaned scenes with no path from an entry point, useful for
    /// detecting disconnected content.
    pub fn unreachable_scenes(&self) -> HashSet<Id<Scene>> {
        let mut visited = HashSet::new();
        let mut stack: Vec<_> = self.roots.iter().collect();

        while let Some(scene) = stack.pop() {
            if visited.insert(scene)
                && let Some(edges) = self.nodes.get(scene)
            {
                stack.extend(edges.iter())
            }
        }

        self.nodes
            .keys()
            .filter(|id| !visited.contains(id))
            .cloned()
            .collect()
    }

    /// Returns an iterator over all scenes reachable from `root`, in
    /// depth-first traversal order (including `root` itself).
    pub fn reachable_from(&self, root: Id<Scene>) -> impl Iterator<Item = Id<Scene>> {
        let mut visited = HashSet::new();
        let mut order = Vec::new();
        let mut stack = vec![root];

        while let Some(current) = stack.pop() {
            if visited.insert(current) {
                order.push(current);
                if let Some(children) = self.nodes.get(&current) {
                    stack.extend(children);
                }
            }
        }

        order.into_iter()
    }

    /// Determines whether `target` is reachable from `start` in the scene graph.
    ///
    /// Performs a depth-first traversal beginning at `start`, following
    /// outgoing edges to check whether `target` appears anywhere downstream.
    /// Used to prevent cycles when moving scenes, and to reason about
    /// ancestor and descendant relationships.
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
    fn is_descendant(&self, start: Id<Scene>, target: Id<Scene>) -> bool {
        let mut visited = HashSet::new();
        let mut stack = vec![start];

        while let Some(node) = stack.pop() {
            if node == target {
                return true;
            }

            if visited.insert(node)
                && let Some(edges) = self.nodes.get(&node)
            {
                stack.extend(edges);
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use crate::models::{
        Id, Scene,
        scene_graph::{Edge, SceneGraph, SceneGraphError, SceneRemovalDetails},
    };

    fn generate_test_components() -> (SceneGraph, Vec<Id<Scene>>) {
        let scene_ids: Vec<Id<Scene>> = (0..3).map(|_| Id::new()).collect();
        let mut scene_graph = SceneGraph::default();
        for id in &scene_ids {
            scene_graph.add_scene(*id);
        }

        scene_graph.add_root(scene_ids[0]).unwrap();

        scene_graph.add_edge(scene_ids[0], scene_ids[1]).unwrap();
        scene_graph.add_edge(scene_ids[1], scene_ids[2]).unwrap();

        (scene_graph, scene_ids)
    }

    #[test]
    fn test_adding_a_scene_works() {
        // ARRANGE
        let (mut graph, _) = generate_test_components();
        let scene_to_add = Id::new();
        // ACT
        let response = graph.add_scene(scene_to_add);
        // ASSERT
        assert!(response)
    }

    #[test]
    fn test_adding_a_scene_that_already_exists_is_a_no_op() {
        // ARRANGE
        let (mut graph, scene_ids) = generate_test_components();
        // ACT
        let response = graph.add_scene(scene_ids[0]);
        // ASSERT
        assert!(!response)
    }

    #[test]
    fn test_removing_a_scene_works() {
        // ARRANGE
        let (mut graph, scene_ids) = generate_test_components();
        let scene_to_remove = scene_ids[1];
        // ACT
        let response = graph.remove_scene(scene_to_remove);
        // ASSERT
        assert!(response.is_some());
        assert_eq!(
            response.unwrap(),
            SceneRemovalDetails {
                edges: vec![
                    Edge {
                        src: scene_ids[1],
                        dest: scene_ids[2]
                    },
                    Edge {
                        src: scene_ids[0],
                        dest: scene_ids[1]
                    }
                ],
                was_root: false
            }
        );
        assert!(!graph.nodes.contains_key(&scene_to_remove))
    }

    #[test]
    fn test_removing_a_scene_that_does_not_exist_is_a_no_op() {
        // ARRANGE
        let (mut graph, _) = generate_test_components();
        let random_id = Id::new();
        // ACT
        let response = graph.remove_scene(random_id);
        // ASSERT
        assert!(response.is_none())
    }

    #[test]
    fn test_cycle_detected_for_invalid_scene_move() {
        // ARRANGE
        let (mut graph, scene_ids) = generate_test_components();
        // ACT
        let response = graph.move_scene(scene_ids[1], scene_ids[0], scene_ids[2]);
        // ASSERT
        assert_eq!(
            response,
            Err(SceneGraphError::CycleDetected {
                scene: scene_ids[1],
                dest: scene_ids[2]
            })
        );
    }

    #[test]
    fn test_rollback_works_when_cycle_detected() {
        // ARRANGE
        let (mut graph, scene_ids) = generate_test_components();
        // ACT
        let response = graph.move_scene(scene_ids[1], scene_ids[0], scene_ids[2]);
        // ASSERT
        assert_eq!(
            response,
            Err(SceneGraphError::CycleDetected {
                scene: scene_ids[1],
                dest: scene_ids[2]
            })
        );
        assert!(
            graph
                .next_scenes(scene_ids[0])
                .collect::<Vec<_>>()
                .contains(&scene_ids[1])
        )
    }

    #[test]
    fn test_cycle_not_detected_for_valid_scene_move() {
        // ARRANGE
        let (mut graph, scene_ids) = generate_test_components();
        // ACT
        let response = graph.move_scene(scene_ids[2], scene_ids[1], scene_ids[0]);
        // ASSERT
        assert!(response.is_ok())
    }

    #[test]
    fn test_move_when_scene_is_not_child_of_src_throws_invalid_move_error() {
        // ARRANGE
        let (mut graph, scene_ids) = generate_test_components();
        // ACT
        let response = graph.move_scene(scene_ids[2], scene_ids[0], scene_ids[1]);
        //ASSERT
        assert_eq!(
            response,
            Err(SceneGraphError::InvalidMove {
                scene: scene_ids[2],
                src: scene_ids[0],
                dest: scene_ids[1]
            })
        )
    }

    #[test]
    fn test_move_with_invalid_scenes_throws_unknown_scene_error() {
        // ARRANGE
        let (mut graph, scene_ids) = generate_test_components();
        let random_scene = Id::new();
        // ACT
        let invalid_scene_move = graph.move_scene(random_scene, scene_ids[0], scene_ids[1]);
        let invalid_src_move = graph.move_scene(scene_ids[0], random_scene, scene_ids[1]);
        let invalid_dest_move = graph.move_scene(scene_ids[0], scene_ids[1], random_scene);
        // ASSERT
        assert_eq!(
            invalid_scene_move,
            Err(SceneGraphError::UnknownScene(random_scene))
        );
        assert_eq!(
            invalid_src_move,
            Err(SceneGraphError::UnknownScene(random_scene))
        );
        assert_eq!(
            invalid_dest_move,
            Err(SceneGraphError::UnknownScene(random_scene))
        );
    }

    #[test]
    fn test_marking_a_valid_scene_as_root_works() {
        // ARRANGE
        let (mut graph, scene_ids) = generate_test_components();
        // ACT
        let response = graph.add_root(scene_ids[2]);
        // ASSERT
        assert!(response.is_ok());
        assert!(response.unwrap());
    }

    #[test]
    fn test_marking_a_scene_already_marked_as_root_is_a_no_op() {
        // ARRANGE
        let (mut graph, scene_ids) = generate_test_components();
        // ACT
        let response = graph.add_root(scene_ids[0]);
        // ASSERT
        assert!(response.is_ok());
        assert!(!response.unwrap());
    }

    #[test]
    fn test_marking_an_invalid_scene_as_root_throws_unknown_scene_error() {
        // ARRANGE
        let (mut graph, _) = generate_test_components();
        let random_id = Id::new();
        // ACT
        let response = graph.add_root(random_id);
        // ASSERT
        assert_eq!(response, Err(SceneGraphError::UnknownScene(random_id)))
    }

    #[test]
    fn test_unmarking_a_valid_scene_as_root_works() {
        // ARRANGE
        let (mut graph, scene_ids) = generate_test_components();
        // ACT
        let response = graph.remove_root(scene_ids[0]);
        // ASSERT
        assert!(response.is_ok());
        assert!(response.unwrap());
    }

    #[test]
    fn test_unmarking_a_valid_scene_not_marked_as_root_is_a_no_op() {
        // ARRANGE
        let (mut graph, scene_ids) = generate_test_components();
        // ACT
        let response = graph.remove_root(scene_ids[2]);
        // ASSERT
        assert!(response.is_ok());
        assert!(!response.unwrap());
    }

    #[test]
    fn test_unmarking_an_invalid_scene_as_root_throws_unknown_scene_error() {
        // ARRANGE
        let (mut graph, _) = generate_test_components();
        let random_id = Id::new();
        // ACT
        let response = graph.remove_root(random_id);
        // ASSERT
        assert_eq!(response, Err(SceneGraphError::UnknownScene(random_id)))
    }

    #[test]
    fn test_adding_an_edge_works() {
        // ARRANGE
        let (mut graph, scene_ids) = generate_test_components();
        // ACT
        let response = graph.add_edge(scene_ids[0], scene_ids[2]);
        //ASSERT
        assert!(response.is_ok());
        assert!(response.unwrap());
    }

    #[test]
    fn test_adding_edge_that_already_exists_is_a_no_op() {
        // ARRANGE
        let (mut graph, scene_ids) = generate_test_components();
        // ACT
        let response = graph.add_edge(scene_ids[0], scene_ids[1]);
        //ASSERT
        assert!(response.is_ok());
        assert!(!response.unwrap());
    }

    #[test]
    fn test_adding_edge_with_invalid_nodes_throws_unknown_scene_error() {
        // ARRANGE
        let (mut graph, scene_ids) = generate_test_components();
        let random_id = Id::new();
        // ACT
        let invalid_src_response = graph.add_edge(random_id, scene_ids[0]);
        let invalid_dest_response = graph.add_edge(scene_ids[1], random_id);
        // ASSERT
        assert_eq!(
            invalid_src_response,
            Err(SceneGraphError::UnknownScene(random_id))
        );
        assert_eq!(
            invalid_dest_response,
            Err(SceneGraphError::UnknownScene(random_id))
        )
    }

    #[test]
    fn test_removing_an_edge_works() {
        // ARRANGE
        let (mut graph, scene_ids) = generate_test_components();
        // ACT
        let response = graph.remove_edge(scene_ids[0], scene_ids[1]);
        //ASSERT
        assert!(response.is_ok());
        assert!(response.unwrap());
    }

    #[test]
    fn test_removing_edge_that_does_not_exist_is_a_no_op() {
        // ARRANGE
        let (mut graph, scene_ids) = generate_test_components();
        // ACT
        let response = graph.remove_edge(scene_ids[0], scene_ids[2]);
        //ASSERT
        assert!(response.is_ok());
        assert!(!response.unwrap());
    }

    #[test]
    fn test_removing_edge_with_invalid_nodes_throws_unknown_scene_error() {
        // ARRANGE
        let (mut graph, scene_ids) = generate_test_components();
        let random_id = Id::new();
        // ACT
        let invalid_src_response = graph.remove_edge(random_id, scene_ids[0]);
        let invalid_dest_response = graph.remove_edge(scene_ids[1], random_id);
        // ASSERT
        assert_eq!(
            invalid_src_response,
            Err(SceneGraphError::UnknownScene(random_id))
        );
        assert_eq!(
            invalid_dest_response,
            Err(SceneGraphError::UnknownScene(random_id))
        )
    }
}
