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
///
/// # Updates
///
/// Every mutation returns the [`SceneGraphUpdate`]s describing what changed.
/// The graph is the sole producer of these; [`Narrative`] forwards them upward
/// so the UI can patch its view incrementally rather than refetching.
///
/// Because they drive that patching, updates report only real changes. An
/// operation whose effect was already achieved returns `None` (or an empty
/// `Vec`) rather than an update for a graph that did not move.
///
/// # Option versus Result
///
/// The two outcomes are distinct and both are represented:
///
/// - `Ok(None)` means the request was valid and its effect already held. Adding
///   an existing variant, linking an already-linked pair, removing an absent
///   edge: nothing to do, nothing to report.
/// - `Err(SceneGraphError)` means the request could not be honored. An
///   unrecognized id, a move that would create a cycle, a variant that is not a
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
/// stay quiet — [`SceneGraph::remove_variant`] calls
/// [`SceneGraph::remove_edge_unchecked`] in loops where absence is expected,
/// and a self-loop legitimately visits the same pair twice.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SceneGraph {
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

    /// Adds a scene variant to the `SceneGraph`, initialized with no outgoing edges.
    ///
    /// Returns `None` if the variant is already in the graph, since nothing changed.
    pub fn add_variant(&mut self, variant_id: Id<SceneVariant>) -> Option<SceneGraphUpdate> {
        match self.edges.entry(variant_id) {
            Entry::Vacant(entry) => {
                entry.insert(HashSet::new());
                Some(SceneGraphUpdate::SceneVariantAdded(variant_id))
            }
            Entry::Occupied(_) => None,
        }
    }

    /// Removes a scene variant from the `SceneGraph`.
    ///
    /// This operation:
    /// - Removes every edge into or out of the variant
    /// - Removes the variant from the set of root entry points, if present
    /// - Removes the variant itself
    ///
    /// Returns one update per change, ordered so that edge and root removals
    /// precede the variant's own removal. Callers patching a rendered graph can
    /// apply them in order without ever referencing a variant that no longer exists.
    ///
    /// Returns an empty `Vec` if the variant is not in the graph.
    pub fn remove_variant(&mut self, variant_id: Id<SceneVariant>) -> Vec<SceneGraphUpdate> {
        let mut variant_deletion_updates = Vec::new();

        // Outgoing
        let outgoing_edges: Vec<_> = self
            .edges
            .get(&variant_id)
            .map(|dests| dests.iter().cloned().collect())
            .unwrap_or_default();

        // Incoming
        let incoming_edges: Vec<_> = self
            .edges
            .iter()
            .filter(|(_, dests)| dests.contains(&variant_id))
            .map(|(src, _)| *src)
            .collect();

        // Delete edges
        variant_deletion_updates.extend(
            outgoing_edges
                .iter()
                .filter_map(|dest| self.remove_edge_unchecked(variant_id, *dest)),
        );

        variant_deletion_updates.extend(
            incoming_edges
                .iter()
                .filter_map(|src| self.remove_edge_unchecked(*src, variant_id)),
        );

        // Remove from roots, if exists
        if self.roots.remove(&variant_id) {
            variant_deletion_updates.push(SceneGraphUpdate::SceneVariantRemovedAsRoot(variant_id))
        }

        // Remove from edges
        if self.edges.remove(&variant_id).is_some() {
            variant_deletion_updates.push(SceneGraphUpdate::SceneVariantRemoved(variant_id));
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
    /// Returns `SceneGraphError::UnknownVariant` if `variant`, `src`, or `dest`
    /// is not present in the graph.
    /// Returns `SceneGraphError::InvalidMove` if `variant` is not a child of `src`.
    /// Returns `SceneGraphError::CycleDetected` if the move would create a cycle.
    /// On either failure the graph is left unchanged.
    pub fn move_variant(
        &mut self,
        variant: Id<SceneVariant>,
        src: Id<SceneVariant>,
        dest: Id<SceneVariant>,
    ) -> Result<SceneGraphUpdate, SceneGraphError> {
        // Verify each node exists in the graph
        for s in [variant, src, dest] {
            if !self.edges.contains_key(&s) {
                return Err(SceneGraphError::UnknownVariant(s));
            }
        }

        if !self
            .edges
            .get_mut(&src)
            .is_some_and(|edges| edges.remove(&variant))
        {
            return Err(SceneGraphError::InvalidMove { variant, src, dest });
        }

        // Argument order matters. This asks whether dest is reachable from variant.
        // The move adds dest -> variant, so a forward path from variant back to dest
        // would close a loop. The reverse question (is variant reachable from dest)
        // only detects a redundant path, which is legal.
        if self.is_descendant(variant, dest) {
            // Getting the edges for the source should always return as Some
            if let Some(edges) = self.edges.get_mut(&src) {
                edges.insert(variant);
            }

            return Err(SceneGraphError::CycleDetected { variant, dest });
        }

        if let Some(edges) = self.edges.get_mut(&dest) {
            edges.insert(variant);
        }

        Ok(SceneGraphUpdate::Move { variant, src, dest })
    }

    /// Marks a scene variant as a root (entry point) in the `SceneGraph`.
    ///
    /// Returns `Ok(None)` if the variant is already a root, since nothing changed.
    ///
    /// # Errors
    ///
    /// Returns `SceneGraphError::UnknownVariant` if the variant is not in the
    /// graph. Roots are never created implicitly: an unrecognized id means the
    /// caller's view of the graph has diverged, which is worth surfacing.
    pub fn add_root(
        &mut self,
        variant_id: Id<SceneVariant>,
    ) -> Result<Option<SceneGraphUpdate>, SceneGraphError> {
        if !self.edges.contains_key(&variant_id) {
            return Err(SceneGraphError::UnknownVariant(variant_id));
        }

        if self.roots.insert(variant_id) {
            return Ok(Some(SceneGraphUpdate::SceneVariantSetAsRoot(variant_id)));
        }

        Ok(None)
    }

    /// Unmarks a scene variant as a root (entry point) in the `SceneGraph`.
    ///
    /// Returns `None` if the variant was not registered as a root.
    pub fn remove_root(
        &mut self,
        variant_id: Id<SceneVariant>,
    ) -> Result<Option<SceneGraphUpdate>, SceneGraphError> {
        if !self.edges.contains_key(&variant_id) {
            return Err(SceneGraphError::UnknownVariant(variant_id));
        }

        if self.roots.remove(&variant_id) {
            return Ok(Some(SceneGraphUpdate::SceneVariantRemovedAsRoot(
                variant_id,
            )));
        }

        Ok(None)
    }

    /// Adds a directed edge from `src` to `dest`, representing a possible next scene.
    ///
    /// Returns `Ok(None)` if the edge already exists, since nothing changed.
    ///
    /// Example: Scene 3 -> Scene 4 or Scene 3 -> Scene 5
    ///
    /// # Errors
    ///
    /// Returns `SceneGraphError::UnknownVariant` if either `src` or `dest` is
    /// not in the graph. Neither is created implicitly.
    pub fn add_edge(
        &mut self,
        src: Id<SceneVariant>,
        dest: Id<SceneVariant>,
    ) -> Result<Option<SceneGraphUpdate>, SceneGraphError> {
        if !self.edges.contains_key(&src) {
            return Err(SceneGraphError::UnknownVariant(src));
        }

        if !self.edges.contains_key(&dest) {
            return Err(SceneGraphError::UnknownVariant(dest));
        }

        if self.edges.get_mut(&src).is_some_and(|e| e.insert(dest)) {
            return Ok(Some(SceneGraphUpdate::EdgeAdded { src, dest }));
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
    /// Returns `SceneGraphError::UnknownVariant` if either `src` or `dest` is
    /// not in the graph. Returns `Ok(None)` if both exist but no edge connects
    /// them, since nothing changed.
    pub fn remove_edge(
        &mut self,
        src: Id<SceneVariant>,
        dest: Id<SceneVariant>,
    ) -> Result<Option<SceneGraphUpdate>, SceneGraphError> {
        if !self.edges.contains_key(&src) {
            return Err(SceneGraphError::UnknownVariant(src));
        }

        if !self.edges.contains_key(&dest) {
            return Err(SceneGraphError::UnknownVariant(dest));
        }

        Ok(self.remove_edge_unchecked(src, dest))
    }

    /// Removes the edge from `src` to `dest` without validating that either
    /// scene variant exists in the graph.
    fn remove_edge_unchecked(
        &mut self,
        src: Id<SceneVariant>,
        dest: Id<SceneVariant>,
    ) -> Option<SceneGraphUpdate> {
        if self.edges.get_mut(&src).is_some_and(|e| e.remove(&dest)) {
            return Some(SceneGraphUpdate::EdgeRemoved { src, dest });
        }

        None
    }

    /// Returns an iterator over all scenes that are direct successors of `variant_id`.  
    /// These represent all possible "next" scenes in the procedural traversal of the graph.
    pub fn next_variants(
        &self,
        variant_id: Id<SceneVariant>,
    ) -> impl Iterator<Item = Id<SceneVariant>> {
        self.edges
            .get(&variant_id)
            .into_iter()
            .flat_map(|set| set.iter().cloned())
    }

    /// Returns all scene variants in the graph that cannot be reached from any root.  
    /// These are "orphaned" scene variant with no path from a root node, useful for detecting disconnected content.
    pub fn unreachable_variants(&self) -> HashSet<Id<SceneVariant>> {
        let mut visited = HashSet::new();
        let mut stack: Vec<_> = self.roots.iter().collect();

        while let Some(variant) = stack.pop() {
            if visited.insert(variant)
                && let Some(edges) = self.edges.get(&variant)
            {
                stack.extend(edges.iter())
            }
        }

        self.edges
            .keys()
            .filter(|id| !visited.contains(id))
            .cloned()
            .collect()
    }

    /// Returns an iterator over all scene variants reachable from `root`, in
    /// depth-first traversal order (including `root` itself).
    pub fn reachable_from<'a>(
        &'a self,
        root: Id<SceneVariant>,
    ) -> impl Iterator<Item = Id<SceneVariant>> {
        let mut visited = HashSet::new();
        let mut order = Vec::new();
        let mut stack = vec![root];

        while let Some(current) = stack.pop() {
            if visited.insert(current) {
                order.push(current);
                if let Some(children) = self.edges.get(&current) {
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
    fn is_descendant(&self, start: Id<SceneVariant>, target: Id<SceneVariant>) -> bool {
        let mut visited = HashSet::new();
        let mut stack = vec![start];

        while let Some(node) = stack.pop() {
            if node == target {
                return true;
            }

            if visited.insert(node)
                && let Some(edges) = self.edges.get(&node)
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
        Id, SceneVariant,
        scene_graph::{SceneGraph, SceneGraphError, SceneGraphUpdate},
    };

    fn generate_test_components() -> (SceneGraph, Vec<Id<SceneVariant>>) {
        let variant_ids: Vec<Id<SceneVariant>> = (0..3).map(|_| Id::new()).collect();
        let mut scene_graph = SceneGraph::default();
        for id in &variant_ids {
            scene_graph.add_variant(*id);
        }

        scene_graph.add_root(variant_ids[0]).unwrap();

        scene_graph
            .add_edge(variant_ids[0], variant_ids[1])
            .unwrap();
        scene_graph
            .add_edge(variant_ids[1], variant_ids[2])
            .unwrap();

        (scene_graph, variant_ids)
    }

    #[test]
    fn test_adding_a_variant_works() {
        // ARRANGE
        let (mut graph, _) = generate_test_components();
        let variant_to_add = Id::new();
        // ACT
        let response = graph.add_variant(variant_to_add);
        // ASSERT
        assert_eq!(
            response,
            Some(SceneGraphUpdate::SceneVariantAdded(variant_to_add))
        )
    }

    #[test]
    fn test_adding_a_variant_that_already_exists_is_a_no_op() {
        // ARRANGE
        let (mut graph, variant_ids) = generate_test_components();
        // ACT
        let response = graph.add_variant(variant_ids[0]);
        // ASSERT
        assert!(response.is_none())
    }

    #[test]
    fn test_removing_a_variant_works() {
        // ARRANGE
        let (mut graph, variant_ids) = generate_test_components();
        let variant_to_remove = variant_ids[1];
        // ACT
        let response = graph.remove_variant(variant_to_remove);
        // ARRANGE
        assert_eq!(response.len(), 3);
        assert_eq!(
            response.last().unwrap(),
            &SceneGraphUpdate::SceneVariantRemoved(variant_to_remove)
        );
        assert!(!graph.edges.contains_key(&variant_to_remove))
    }

    #[test]
    fn test_removing_a_variant_that_does_not_exist_is_a_no_op() {
        // ARRANGE
        let (mut graph, _) = generate_test_components();
        let random_id = Id::new();
        // ACT
        let response = graph.remove_variant(random_id);
        // ASSERT
        assert_eq!(response.len(), 0)
    }

    #[test]
    fn test_cycle_detected_for_invalid_variant_move() {
        // ARRANGE
        let (mut graph, variant_ids) = generate_test_components();
        // ACT
        let response = graph.move_variant(variant_ids[1], variant_ids[0], variant_ids[2]);
        // ASSERT
        assert_eq!(
            response,
            Err(SceneGraphError::CycleDetected {
                variant: variant_ids[1],
                dest: variant_ids[2]
            })
        );
    }

    #[test]
    fn test_rollback_works_when_cycle_detected() {
        // ARRANGE
        let (mut graph, variant_ids) = generate_test_components();
        // ACT
        let response = graph.move_variant(variant_ids[1], variant_ids[0], variant_ids[2]);
        // ASSERT
        assert_eq!(
            response,
            Err(SceneGraphError::CycleDetected {
                variant: variant_ids[1],
                dest: variant_ids[2]
            })
        );
        assert!(
            graph
                .next_variants(variant_ids[0])
                .collect::<Vec<_>>()
                .contains(&variant_ids[1])
        )
    }

    #[test]
    fn test_cycle_not_detected_for_valid_variant_move() {
        // ARRANGE
        let (mut graph, variant_ids) = generate_test_components();
        // ACT
        let response = graph.move_variant(variant_ids[2], variant_ids[1], variant_ids[0]);
        // ASSERT
        assert_eq!(
            response,
            Ok(SceneGraphUpdate::Move {
                variant: variant_ids[2],
                src: variant_ids[1],
                dest: variant_ids[0]
            })
        );
    }

    #[test]
    fn test_move_when_variant_is_not_child_of_src_throws_invalid_move_error() {
        // ARRANGE
        let (mut graph, variant_ids) = generate_test_components();
        // ACT
        let response = graph.move_variant(variant_ids[2], variant_ids[0], variant_ids[1]);
        //ASSERT
        assert_eq!(
            response,
            Err(SceneGraphError::InvalidMove {
                variant: variant_ids[2],
                src: variant_ids[0],
                dest: variant_ids[1]
            })
        )
    }

    #[test]
    fn test_move_with_invalid_variants_throws_unknown_variant_error() {
        // ARRANGE
        let (mut graph, variant_ids) = generate_test_components();
        let random_variant = Id::new();
        // ACT
        let invalid_variant_move =
            graph.move_variant(random_variant, variant_ids[0], variant_ids[1]);
        let invalid_src_move = graph.move_variant(variant_ids[0], random_variant, variant_ids[1]);
        let invalid_dest_move = graph.move_variant(variant_ids[0], variant_ids[1], random_variant);
        // ASSERT
        assert_eq!(
            invalid_variant_move,
            Err(SceneGraphError::UnknownVariant(random_variant))
        );
        assert_eq!(
            invalid_src_move,
            Err(SceneGraphError::UnknownVariant(random_variant))
        );
        assert_eq!(
            invalid_dest_move,
            Err(SceneGraphError::UnknownVariant(random_variant))
        );
    }

    #[test]
    fn test_marking_a_valid_variant_as_root_works() {
        // ARRANGE
        let (mut graph, variant_ids) = generate_test_components();
        // ACT
        let response = graph.add_root(variant_ids[2]);
        // ASSERT
        assert_eq!(
            response,
            Ok(Some(SceneGraphUpdate::SceneVariantSetAsRoot(
                variant_ids[2]
            )))
        )
    }

    #[test]
    fn test_marking_a_variant_already_marked_as_root_is_a_no_op() {
        // ARRANGE
        let (mut graph, variant_ids) = generate_test_components();
        // ACT
        let response = graph.add_root(variant_ids[0]);
        // ASSERT
        assert_eq!(response, Ok(None))
    }

    #[test]
    fn test_marking_an_invalid_variant_as_root_throws_unknown_variant_error() {
        // ARRANGE
        let (mut graph, _) = generate_test_components();
        let random_id = Id::new();
        // ACT
        let response = graph.add_root(random_id);
        // ASSERT
        assert_eq!(response, Err(SceneGraphError::UnknownVariant(random_id)))
    }

    #[test]
    fn test_unmarking_a_valid_variant_as_root_works() {
        // ARRANGE
        let (mut graph, variant_ids) = generate_test_components();
        // ACT
        let response = graph.remove_root(variant_ids[0]);
        // ASSERT
        assert_eq!(
            response,
            Ok(Some(SceneGraphUpdate::SceneVariantRemovedAsRoot(
                variant_ids[0]
            )))
        )
    }

    #[test]
    fn test_unmarking_a_valid_variant_not_marked_as_root_is_a_no_op() {
        // ARRANGE
        let (mut graph, variant_ids) = generate_test_components();
        // ACT
        let response = graph.remove_root(variant_ids[2]);
        // ASSERT
        assert_eq!(response, Ok(None))
    }

    #[test]
    fn test_unmarking_an_invalid_variant_as_root_throws_unknown_variant_error() {
        // ARRANGE
        let (mut graph, _) = generate_test_components();
        let random_id = Id::new();
        // ACT
        let response = graph.remove_root(random_id);
        // ASSERT
        assert_eq!(response, Err(SceneGraphError::UnknownVariant(random_id)))
    }

    #[test]
    fn test_adding_an_edge_works() {
        // ARRANGE
        let (mut graph, variant_ids) = generate_test_components();
        // ACT
        let response = graph.add_edge(variant_ids[0], variant_ids[2]);
        //ASSERT
        assert_eq!(
            response,
            Ok(Some(SceneGraphUpdate::EdgeAdded {
                src: variant_ids[0],
                dest: variant_ids[2]
            }))
        )
    }

    #[test]
    fn test_adding_edge_that_already_exists_is_a_no_op() {
        // ARRANGE
        let (mut graph, variant_ids) = generate_test_components();
        // ACT
        let response = graph.add_edge(variant_ids[0], variant_ids[1]);
        //ASSERT
        assert_eq!(response, Ok(None))
    }

    #[test]
    fn test_adding_edge_with_invalid_nodes_throws_unknown_variant_error() {
        // ARRANGE
        let (mut graph, variant_ids) = generate_test_components();
        let random_id = Id::new();
        // ACT
        let invalid_src_response = graph.add_edge(random_id, variant_ids[0]);
        let invalid_dest_response = graph.add_edge(variant_ids[1], random_id);
        // ASSERT
        assert_eq!(
            invalid_src_response,
            Err(SceneGraphError::UnknownVariant(random_id))
        );
        assert_eq!(
            invalid_dest_response,
            Err(SceneGraphError::UnknownVariant(random_id))
        )
    }

    #[test]
    fn test_removing_an_edge_works() {
        // ARRANGE
        let (mut graph, variant_ids) = generate_test_components();
        // ACT
        let response = graph.remove_edge(variant_ids[0], variant_ids[1]);
        //ASSERT
        assert_eq!(
            response,
            Ok(Some(SceneGraphUpdate::EdgeRemoved {
                src: variant_ids[0],
                dest: variant_ids[1]
            }))
        )
    }

    #[test]
    fn test_removing_edge_that_does_not_exist_is_a_no_op() {
        // ARRANGE
        let (mut graph, variant_ids) = generate_test_components();
        // ACT
        let response = graph.remove_edge(variant_ids[0], variant_ids[2]);
        //ASSERT
        assert_eq!(response, Ok(None))
    }

    #[test]
    fn test_removing_edge_with_invalid_nodes_throws_unknown_variant_error() {
        // ARRANGE
        let (mut graph, variant_ids) = generate_test_components();
        let random_id = Id::new();
        // ACT
        let invalid_src_response = graph.remove_edge(random_id, variant_ids[0]);
        let invalid_dest_response = graph.remove_edge(variant_ids[1], random_id);
        // ASSERT
        assert_eq!(
            invalid_src_response,
            Err(SceneGraphError::UnknownVariant(random_id))
        );
        assert_eq!(
            invalid_dest_response,
            Err(SceneGraphError::UnknownVariant(random_id))
        )
    }
}
