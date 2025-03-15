//! # Core dialogue graph structure.
//!
//! This module defines the `DialogueGraph` struct, which represents a complete dialogue
//! with its nodes, connections, and metadata.

use bevy::prelude::*;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::{EdgeRef, IntoNodeReferences};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::node::NodeId;
use super::nodes::DialogueNode;
use super::{Connection, DialogueElement};

/// Represents a complete dialogue graph with nodes and metadata.
///
/// `DialogueGraph` is the core data structure that contains all elements of a dialogue:
///
/// - Nodes of various types (text, choice, etc.)
/// - Connections between nodes that define the flow
/// - Metadata such as the name and starting point
///
/// Internally, the graph uses `petgraph` for efficient graph operations while
/// maintaining a more dialogue-specific API for client code.
///
/// # Structure
///
/// - `graph`: The underlying petgraph directed graph
/// - `node_indices`: Mapping from NodeId to petgraph NodeIndex
/// - `start_node`: The starting node ID for this dialogue
/// - `name`: Optional name or identifier for this dialogue
///
/// # Example
///
/// ```rust
/// use funkus_dialogue::graph::{DialogueGraph, NodeId, DialogueNode};
///
/// // Create a new dialogue graph
/// let mut graph = DialogueGraph::new(NodeId(1))
///     .with_name("Simple Dialogue");
///
/// // Add a text node
/// let text_node = DialogueNode::text(NodeId(1), "Hello adventurer!")
///     .with_speaker("Guide");
/// graph.add_node(text_node);
///
/// // Add a choice node
/// let choice_node = DialogueNode::choice(NodeId(2))
///     .with_prompt("How do you respond?").unwrap()
///     .with_choice("Greet back", NodeId(3)).unwrap()
///     .with_choice("Ignore", NodeId(4)).unwrap();
/// graph.add_node(choice_node);
///
/// // Add response nodes
/// graph.add_node(DialogueNode::text(NodeId(3), "Nice to meet you too!"));
/// graph.add_node(DialogueNode::text(NodeId(4), "..."));
///
/// // Connect nodes
/// graph.add_edge(NodeId(1), NodeId(2), None).unwrap();
///
/// // Traversing the graph
/// let start = graph.get_start_node().unwrap();
/// let next_nodes = graph.get_connected_nodes(start.id());
///
/// // Removing a node
/// graph.remove_node(NodeId(4)).unwrap();
/// ```
#[derive(Debug, Clone, Reflect)]
pub struct DialogueGraph {
    /// The underlying directed graph - primary data store for nodes and connections
    #[reflect(ignore)]
    graph: DiGraph<DialogueNode, Option<String>>,
    /// Mapping between our stable NodeIds and petgraph's internal NodeIndices.
    /// This map is essential because:
    /// 1. Petgraph's indices may change during operations like node removal
    /// 2. It lets us use consistent, stable identifiers in the public API and serialized data
    /// 3. It provides O(1) lookups when translating between our IDs and petgraph's indices
    #[reflect(ignore)]
    node_indices: HashMap<NodeId, NodeIndex>,
    /// The starting node ID for this dialogue
    pub start_node: NodeId,
    /// Optional name or identifier for this dialogue
    pub name: Option<String>,
}

impl Serialize for DialogueGraph {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Define a serialization structure that directly mirrors our petgraph approach
        // Using Vec<DialogueNode> instead of HashMap<NodeId, DialogueNode> because:
        // 1. It directly represents how nodes are stored in petgraph
        // 2. Each DialogueNode already contains its ID and connections
        // 3. It simplifies serialization/deserialization logic
        #[derive(Serialize)]
        struct SerialGraph {
            // Store nodes as a flat array - their relationships are defined by their connections
            nodes: Vec<DialogueNode>,
            start_node: NodeId,
            name: Option<String>,
        }

        // Collect all nodes from petgraph's node_weights iterator into a Vec
        let nodes: Vec<DialogueNode> = self.graph.node_weights().cloned().collect();

        let graph_data = SerialGraph {
            nodes,
            start_node: self.start_node,
            name: self.name.clone(),
        };

        graph_data.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DialogueGraph {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Define a matching deserialization structure to receive the array-based format
        #[derive(Deserialize)]
        struct SerialGraph {
            nodes: Vec<DialogueNode>,
            start_node: NodeId,
            name: Option<String>,
        }

        let data = SerialGraph::deserialize(deserializer)?;

        // Create a new graph with the basic properties
        let mut graph = DialogueGraph::new(data.start_node);
        graph.name = data.name;

        // First, add all nodes to the graph
        for node in &data.nodes {
            graph.add_node(node.clone());
        }

        // Then, add edges directly to the petgraph structure without modifying nodes' internal connections
        for node in &data.nodes {
            let from_id = node.id();
            if let Some(&from_idx) = graph.node_indices.get(&from_id) {
                for conn in node.connections() {
                    let to_id = conn.target_id;
                    if let Some(&to_idx) = graph.node_indices.get(&to_id) {
                        graph.graph.add_edge(from_idx, to_idx, conn.label.clone());
                    }
                }
            }
        }

        Ok(graph)
    }
}

impl DialogueGraph {
    /// Creates a new empty dialogue graph with the specified start node ID.
    ///
    /// # Parameters
    ///
    /// * `start_node` - The ID of the node that will be the starting point for this dialogue
    ///
    /// # Returns
    ///
    /// A new, empty DialogueGraph with the specified start node
    ///
    /// # Example
    ///
    /// ```rust
    /// use funkus_dialogue::graph::{DialogueGraph, NodeId};
    ///
    /// let graph = DialogueGraph::new(NodeId(1));
    /// assert_eq!(graph.start_node, NodeId(1));
    /// ```
    pub fn new(start_node: NodeId) -> Self {
        Self {
            graph: DiGraph::new(),
            node_indices: HashMap::new(),
            start_node,
            name: None,
        }
    }

    /// Sets the name of this dialogue graph.
    ///
    /// # Parameters
    ///
    /// * `name` - The name to assign to this dialogue graph
    ///
    /// # Returns
    ///
    /// The dialogue graph with the name set
    ///
    /// # Example
    ///
    /// ```rust
    /// use funkus_dialogue::graph::{DialogueGraph, NodeId};
    ///
    /// let graph = DialogueGraph::new(NodeId(1))
    ///     .with_name("Tutorial Dialogue");
    ///     
    /// assert_eq!(graph.name, Some("Tutorial Dialogue".to_string()));
    /// ```
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Adds a node to the graph.
    ///
    /// This method adds a node to the petgraph structure and updates the node_indices map
    /// to maintain the mapping between NodeId and petgraph's internal NodeIndex.
    ///
    /// # Parameters
    ///
    /// * `node` - The node to add to the graph
    ///
    /// # Example
    ///
    /// ```rust
    /// use funkus_dialogue::graph::{DialogueGraph, NodeId, DialogueNode};
    ///
    /// let mut graph = DialogueGraph::new(NodeId(1));
    /// let text_node = DialogueNode::text(NodeId(1), "Hello, world!");
    ///
    /// graph.add_node(text_node);
    /// ```
    pub fn add_node(&mut self, node: DialogueNode) {
        let id = node.id();
        let index = self.graph.add_node(node);
        self.node_indices.insert(id, index);
    }

    /// Adds a node to the graph using builder pattern.
    ///
    /// # Parameters
    ///
    /// * `node` - The node to add to the graph
    ///
    /// # Returns
    ///
    /// The dialogue graph with the node added
    ///
    /// # Example
    ///
    /// ```rust
    /// use funkus_dialogue::graph::{DialogueGraph, NodeId, DialogueNode};
    ///
    /// let text_node = DialogueNode::text(NodeId(1), "Hello, world!");
    ///
    /// let graph = DialogueGraph::new(NodeId(1))
    ///     .with_node(text_node);
    /// ```
    pub fn with_node(mut self, node: DialogueNode) -> Self {
        self.add_node(node);
        self
    }

    /// Add an edge (connection) between nodes.
    ///
    /// This method creates a directed edge from one node to another, optionally with a label.
    /// It translates the NodeIds to petgraph's internal NodeIndices before adding the edge.
    ///
    /// # Parameters
    ///
    /// * `from_id` - The ID of the source node
    /// * `to_id` - The ID of the target node
    /// * `label` - Optional label for this connection
    ///
    /// # Returns
    ///
    /// Ok(()) if the edge was added successfully, or an error string if either node doesn't exist
    ///
    /// # Example
    ///
    /// ```rust
    /// use funkus_dialogue::graph::{DialogueGraph, NodeId, DialogueNode};
    ///
    /// let mut graph = DialogueGraph::new(NodeId(1));
    ///
    /// // Add two nodes
    /// graph.add_node(DialogueNode::text(NodeId(1), "First node"));
    /// graph.add_node(DialogueNode::text(NodeId(2), "Second node"));
    ///
    /// // Connect them
    /// let result = graph.add_edge(NodeId(1), NodeId(2), Some("Next".to_string()));
    /// assert!(result.is_ok());
    /// ```
    pub fn add_edge(
        &mut self,
        from_id: NodeId,
        to_id: NodeId,
        label: Option<String>,
    ) -> Result<(), String> {
        let from_index = self
            .node_indices
            .get(&from_id)
            .ok_or_else(|| format!("Source node {:?} not found", from_id))?;
        let to_index = self
            .node_indices
            .get(&to_id)
            .ok_or_else(|| format!("Target node {:?} not found", to_id))?;

        // Add the edge to the graph
        self.graph.add_edge(*from_index, *to_index, label.clone());

        // Also update the node's internal connections field to keep it in sync
        if let Some(node) = self.graph.node_weight_mut(*from_index) {
            // Create connection
            let connection = Connection {
                target_id: to_id,
                label: label.clone(),
            };

            // Add to the node's connections array
            match node {
                DialogueNode::Text { connections, .. } => connections.push(connection),
                DialogueNode::Choice { connections, .. } => connections.push(connection),
            }
        }

        Ok(())
    }

    /// Gets a node by its ID.
    ///
    /// This method translates the NodeId to petgraph's internal NodeIndex
    /// and then retrieves the node from the graph.
    ///
    /// # Parameters
    ///
    /// * `id` - The ID of the node to retrieve
    ///
    /// # Returns
    ///
    /// An optional reference to the node if it exists, or None if not found
    ///
    /// # Example
    ///
    /// ```rust
    /// use funkus_dialogue::graph::{DialogueGraph, NodeId, DialogueNode};
    ///
    /// let mut graph = DialogueGraph::new(NodeId(1));
    /// graph.add_node(DialogueNode::text(NodeId(1), "Hello"));
    ///
    /// let node = graph.get_node(NodeId(1));
    /// assert!(node.is_some());
    ///
    /// let missing_node = graph.get_node(NodeId(99));
    /// assert!(missing_node.is_none());
    /// ```
    pub fn get_node(&self, id: NodeId) -> Option<&DialogueNode> {
        // Get the NodeIndex for this NodeId and then look up the node in the graph
        self.node_indices
            .get(&id)
            .and_then(|&idx| self.graph.node_weight(idx))
    }

    /// Gets a mutable reference to a node by its ID.
    ///
    /// Similar to get_node, but returns a mutable reference, allowing the node to be modified.
    ///
    /// # Parameters
    ///
    /// * `id` - The ID of the node to retrieve
    ///
    /// # Returns
    ///
    /// An optional mutable reference to the node if it exists, or None if not found
    pub fn get_node_mut(&mut self, id: NodeId) -> Option<&mut DialogueNode> {
        self.node_indices
            .get(&id)
            .and_then(|&idx| self.graph.node_weight_mut(idx))
    }

    /// Gets the starting node of the dialogue.
    ///
    /// # Returns
    ///
    /// An optional reference to the start node if it exists, or None if the start node ID is invalid
    ///
    /// # Example
    ///
    /// ```rust
    /// use funkus_dialogue::graph::{DialogueGraph, NodeId, DialogueNode};
    ///
    /// let mut graph = DialogueGraph::new(NodeId(1));
    /// graph.add_node(DialogueNode::text(NodeId(1), "Start node"));
    ///
    /// let start_node = graph.get_start_node();
    /// assert!(start_node.is_some());
    /// ```
    pub fn get_start_node(&self) -> Option<&DialogueNode> {
        self.get_node(self.start_node)
    }

    /// Validates the graph structure.
    ///
    /// This performs several checks to ensure the graph is valid:
    /// - All node connections reference valid nodes
    /// - The start node exists
    /// - All nodes are reachable from the start node
    ///
    /// # Returns
    ///
    /// Ok(()) if the graph is valid, or an error message describing the issue
    pub fn validate(&self) -> Result<(), String> {
        // Check for connections to invalid nodes
        for node_idx in self.graph.node_indices() {
            if let Some(node) = self.graph.node_weight(node_idx) {
                for conn in node.connections() {
                    if !self.node_indices.contains_key(&conn.target_id) {
                        return Err(format!(
                            "Node {:?} references non-existent node {:?}",
                            node.id(),
                            conn.target_id
                        ));
                    }
                }
            }
        }

        // Check that the start node exists
        if !self.node_indices.contains_key(&self.start_node) {
            return Err(format!("Start node {:?} does not exist", self.start_node));
        }

        // Check for unreachable nodes using petgraph's algorithms
        if let Some(&start_index) = self.node_indices.get(&self.start_node) {
            // Using Petgraph's reachability analysis
            for (node_id, &node_idx) in &self.node_indices {
                if *node_id != self.start_node {
                    let reachable = petgraph::algo::has_path_connecting(
                        &self.graph,
                        start_index,
                        node_idx,
                        None,
                    );
                    if !reachable {
                        return Err(format!("Node {:?} is unreachable from start node", node_id));
                    }
                }
            }
        }

        Ok(())
    }

    /// Get all nodes connected to the given node.
    ///
    /// This method returns a list of NodeIds and optional connection labels for all
    /// nodes that are direct targets of outgoing edges from the specified node.
    ///
    /// # Parameters
    ///
    /// * `id` - The ID of the node to find connections from
    ///
    /// # Returns
    ///
    /// A vector of (NodeId, Option<String>) pairs representing connected nodes and their connection labels
    pub fn get_connected_nodes(&self, id: NodeId) -> Vec<(NodeId, Option<String>)> {
        if let Some(&node_idx) = self.node_indices.get(&id) {
            let edges = self
                .graph
                .edges_directed(node_idx, petgraph::Direction::Outgoing);
            edges
                .filter_map(|edge| {
                    let target_idx = edge.target();
                    // Find NodeId for this target using node_indices in reverse
                    let target_id = self.node_indices.iter().find_map(|(id, &idx)| {
                        if idx == target_idx {
                            Some(*id)
                        } else {
                            None
                        }
                    })?;

                    Some((target_id, edge.weight().clone()))
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Returns the number of nodes in the graph.
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Returns all node IDs in the graph.
    pub fn node_ids(&self) -> Vec<NodeId> {
        self.node_indices.keys().cloned().collect()
    }

    /// Returns an iterator over all nodes in the graph.
    pub fn nodes_iter(&self) -> impl Iterator<Item = &DialogueNode> {
        self.graph.node_weights()
    }

    /// Checks if a node with the specified ID exists in the graph.
    pub fn contains_node(&self, id: NodeId) -> bool {
        self.node_indices.contains_key(&id)
    }

    /// Updates a node in the graph.
    ///
    /// This method allows modifying a node that's already in the graph.
    ///
    /// # Parameters
    ///
    /// * `id` - The ID of the node to update
    /// * `node` - The new node data
    ///
    /// # Returns
    ///
    /// Ok(()) if the update was successful, or an error if the node doesn't exist
    pub fn update_node(&mut self, id: NodeId, node: DialogueNode) -> Result<(), String> {
        if let Some(&idx) = self.node_indices.get(&id) {
            if let Some(existing_node) = self.graph.node_weight_mut(idx) {
                *existing_node = node;
                Ok(())
            } else {
                Err(format!("Node {:?} found in indices but not in graph", id))
            }
        } else {
            Err(format!("Node {:?} not found", id))
        }
    }

    /// Removes a node from the graph.
    ///
    /// This method removes a node and all its incoming and outgoing connections.
    /// It properly maintains the NodeId-to-NodeIndex mapping by accounting for
    /// petgraph's node removal behavior, which may reindex other nodes.
    ///
    /// # Parameters
    ///
    /// * `id` - The ID of the node to remove
    ///
    /// # Returns
    ///
    /// Ok(()) if the removal was successful, or an error if the node doesn't exist
    pub fn remove_node(&mut self, id: NodeId) -> Result<(), String> {
        if let Some(&idx) = self.node_indices.get(&id) {
            // Before removing the node, check if it's the last node
            let is_last_node = idx.index() == self.graph.node_count() - 1;

            // If it's not the last node, find which node will be moved to its position
            let last_node_id = if !is_last_node {
                // Find the ID of the last node that will be moved
                let last_idx = NodeIndex::new(self.graph.node_count() - 1);
                let last_id = self
                    .node_indices
                    .iter()
                    .find_map(|(&nid, &nidx)| if nidx == last_idx { Some(nid) } else { None })
                    .ok_or_else(|| "Failed to find last node ID".to_string())?;
                Some(last_id)
            } else {
                None
            };

            // Remove the node from petgraph
            self.graph.remove_node(idx);

            // Remove the mapping for the deleted node
            self.node_indices.remove(&id);

            // Update the mapping for the last node that was moved
            if let Some(last_id) = last_node_id {
                // The last node now has the index of the removed node
                self.node_indices.insert(last_id, idx);
            }

            Ok(())
        } else {
            Err(format!("Node {:?} not found", id))
        }
    }

    /// Rebuilds the NodeId-to-NodeIndex mapping.
    ///
    /// This is useful after operations that might have invalidated the mapping
    /// or if you suspect the mapping might be inconsistent with the graph.
    pub fn rebuild_mapping(&mut self) {
        // Clear existing mapping
        self.node_indices.clear();

        // Rebuild from current graph state
        for (idx, node) in self.graph.node_references() {
            self.node_indices.insert(node.id(), idx);
        }
    }

    /// Validates that the NodeId-to-NodeIndex mapping is consistent with the graph.
    ///
    /// This method is available in debug builds to check for mapping inconsistencies.
    ///
    /// # Returns
    ///
    /// Ok(()) if the mapping is valid, or an error message if inconsistencies are found
    #[cfg(debug_assertions)]
    pub fn validate_mapping(&self) -> Result<(), String> {
        // Check that all nodes in the graph have an entry in the mapping

        use petgraph::visit::IntoNodeReferences;
        for (idx, node) in self.graph.node_references() {
            let id = node.id();
            match self.node_indices.get(&id) {
                Some(&mapped_idx) if mapped_idx == idx => {
                    // This mapping is correct
                }
                Some(&mapped_idx) => {
                    return Err(format!(
                        "Inconsistent mapping: Node {:?} has index {:?} in graph but {:?} in mapping",
                        id, idx, mapped_idx
                    ));
                }
                None => {
                    return Err(format!(
                        "Missing mapping: Node {:?} at index {:?} has no mapping entry",
                        id, idx
                    ));
                }
            }
        }

        // Check that all entries in the mapping correspond to nodes in the graph
        for (&id, &idx) in &self.node_indices {
            if let Some(node) = self.graph.node_weight(idx) {
                if node.id() != id {
                    return Err(format!(
                        "Invalid mapping: NodeId {:?} maps to index {:?}, but that index contains NodeId {:?}",
                        id, idx, node.id()
                    ));
                }
            } else {
                return Err(format!(
                    "Stale mapping: NodeId {:?} maps to index {:?}, but that index doesn't exist in the graph",
                    id, idx
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create a basic test graph
    fn create_test_graph() -> DialogueGraph {
        let mut graph = DialogueGraph::new(NodeId(1));

        // Add a few nodes
        graph.add_node(DialogueNode::text(NodeId(1), "Start").with_speaker("NPC"));

        graph.add_node(
            DialogueNode::choice(NodeId(2))
                .with_prompt("Choose:")
                .unwrap()
                .with_choice("Option A", NodeId(3))
                .unwrap()
                .with_choice("Option B", NodeId(4))
                .unwrap(),
        );

        graph.add_node(DialogueNode::text(NodeId(3), "You chose A"));
        graph.add_node(DialogueNode::text(NodeId(4), "You chose B"));

        // Connect nodes
        graph.add_edge(NodeId(1), NodeId(2), None).unwrap();

        graph
    }

    #[test]
    fn test_create_graph() {
        let graph = DialogueGraph::new(NodeId(5)).with_name("Test Graph");

        assert_eq!(graph.start_node, NodeId(5));
        assert_eq!(graph.name, Some("Test Graph".to_string()));
        assert_eq!(graph.node_count(), 0);
    }

    #[test]
    fn test_add_nodes() {
        let mut graph = DialogueGraph::new(NodeId(1));

        graph.add_node(DialogueNode::text(NodeId(1), "Node 1"));
        assert_eq!(graph.node_count(), 1);

        let node = graph.get_node(NodeId(1)).unwrap();
        if let DialogueNode::Text { text, .. } = node {
            assert_eq!(text, "Node 1");
        } else {
            panic!("Expected Text node");
        }

        // Test builder pattern
        let graph = graph.with_node(DialogueNode::text(NodeId(2), "Node 2"));
        assert_eq!(graph.node_count(), 2);
    }

    #[test]
    fn test_connections() {
        let graph = create_test_graph();

        // Test connected nodes
        let connections = graph.get_connected_nodes(NodeId(1));
        assert_eq!(connections.len(), 1);
        assert_eq!(connections[0].0, NodeId(2));

        // Test get_start_node
        let start = graph.get_start_node().unwrap();
        if let DialogueNode::Text { speaker, .. } = start {
            assert_eq!(speaker.clone(), Some("NPC".to_string())); // Fixed: clone the Option<String>
        } else {
            panic!("Expected Text node");
        }
    }

    #[test]
    fn test_node_access() {
        let mut graph = create_test_graph();

        // Test contains_node
        assert!(graph.contains_node(NodeId(1)));
        assert!(!graph.contains_node(NodeId(99)));

        // Test get_node_mut
        if let Some(node) = graph.get_node_mut(NodeId(3)) {
            if let DialogueNode::Text { text, .. } = node {
                *text = "Modified text".to_string();
            }
        }

        let modified = graph.get_node(NodeId(3)).unwrap();
        if let DialogueNode::Text { text, .. } = modified {
            assert_eq!(text, "Modified text");
        } else {
            panic!("Expected Text node");
        }
    }

    #[test]
    fn test_update_node() {
        let mut graph = create_test_graph();

        let new_node = DialogueNode::text(NodeId(3), "Updated node");
        graph.update_node(NodeId(3), new_node).unwrap();

        let updated = graph.get_node(NodeId(3)).unwrap();
        if let DialogueNode::Text { text, .. } = updated {
            assert_eq!(text, "Updated node");
        } else {
            panic!("Expected Text node");
        }

        // Test updating non-existent node
        assert!(graph
            .update_node(NodeId(99), DialogueNode::text(NodeId(99), "Bad"))
            .is_err());
    }

    #[test]
    fn test_node_removal() {
        let mut graph = create_test_graph();
        assert_eq!(graph.node_count(), 4);

        // Remove a node in the middle
        graph.remove_node(NodeId(2)).unwrap();
        assert_eq!(graph.node_count(), 3);
        assert!(!graph.contains_node(NodeId(2)));

        // Ensure we can still access other nodes
        assert!(graph.get_node(NodeId(1)).is_some());
        assert!(graph.get_node(NodeId(3)).is_some());
        assert!(graph.get_node(NodeId(4)).is_some());

        // Test removing the last node
        let last_id = NodeId(4);
        graph.remove_node(last_id).unwrap();
        assert_eq!(graph.node_count(), 2);
        assert!(!graph.contains_node(last_id));

        // Test error on removing non-existent node
        assert!(graph.remove_node(NodeId(99)).is_err());
    }

    #[test]
    fn test_node_removal_swapping() {
        // This test specifically verifies the index swapping behavior
        let mut graph = DialogueGraph::new(NodeId(1));

        // Add nodes in sequence
        for i in 1..=5 {
            graph.add_node(DialogueNode::text(NodeId(i), format!("Node {}", i)));
        }

        // Remove node 2 - this should cause node 5 to be moved to index 1
        graph.remove_node(NodeId(2)).unwrap();

        // Verify we can still access node 5
        let node5 = graph.get_node(NodeId(5));
        assert!(node5.is_some());

        // Add a connection that uses node 5
        graph.add_edge(NodeId(1), NodeId(5), None).unwrap();

        // Verify the connection works
        let connections = graph.get_connected_nodes(NodeId(1));
        assert_eq!(connections.len(), 1);
        assert_eq!(connections[0].0, NodeId(5));

        // Validate the mapping explicitly
        #[cfg(debug_assertions)]
        graph.validate_mapping().unwrap();
    }

    #[test]
    fn test_graph_validation() {
        let mut graph = DialogueGraph::new(NodeId(1));

        // Empty graph should fail validation (start node doesn't exist)
        assert!(graph.validate().is_err());

        // Add start node
        graph.add_node(DialogueNode::text(NodeId(1), "Start"));
        assert!(graph.validate().is_ok());

        // Add unreachable node - should fail validation
        graph.add_node(DialogueNode::text(NodeId(2), "Unreachable"));
        assert!(graph.validate().is_err());

        // Connect nodes - should pass validation
        graph.add_edge(NodeId(1), NodeId(2), None).unwrap();
        assert!(graph.validate().is_ok());
    }

    #[test]
    fn test_rebuild_mapping() {
        let mut graph = create_test_graph();

        // Extract text values from nodes before rebuilding mapping
        let text1 = if let DialogueNode::Text { text, .. } = graph.get_node(NodeId(1)).unwrap() {
            text.clone()
        } else {
            panic!("Expected Text node");
        };

        let text3 = if let DialogueNode::Text { text, .. } = graph.get_node(NodeId(3)).unwrap() {
            text.clone()
        } else {
            panic!("Expected Text node");
        };

        // Rebuild the mapping
        graph.rebuild_mapping();

        // Verify nodes are still accessible with the same content
        if let DialogueNode::Text { text, .. } = graph.get_node(NodeId(1)).unwrap() {
            assert_eq!(text, &text1);
        } else {
            panic!("Node type mismatch");
        }

        if let DialogueNode::Text { text, .. } = graph.get_node(NodeId(3)).unwrap() {
            assert_eq!(text, &text3);
        } else {
            panic!("Node type mismatch");
        }

        // Validate mapping consistency
        #[cfg(debug_assertions)]
        graph.validate_mapping().unwrap();
    }

    #[test]
    fn test_node_iteration() {
        let graph = create_test_graph();

        // Test nodes_iter
        let nodes: Vec<_> = graph.nodes_iter().collect();
        assert_eq!(nodes.len(), 4);

        // Test node_ids
        let ids = graph.node_ids();
        assert_eq!(ids.len(), 4);
        assert!(ids.contains(&NodeId(1)));
        assert!(ids.contains(&NodeId(2)));
        assert!(ids.contains(&NodeId(3)));
        assert!(ids.contains(&NodeId(4)));
    }

    #[test]
    fn test_serialization() {
        use serde_json;

        let original = create_test_graph();

        // Serialize to JSON
        let json = serde_json::to_string(&original).unwrap();

        // Deserialize back to a graph
        let deserialized: DialogueGraph = serde_json::from_str(&json).unwrap();

        // Verify structure is preserved
        assert_eq!(deserialized.start_node, original.start_node);
        assert_eq!(deserialized.node_count(), original.node_count());

        // Check node contents - extract text to avoid borrowing issues
        let original_text =
            if let DialogueNode::Text { text, .. } = original.get_node(NodeId(1)).unwrap() {
                text.clone()
            } else {
                panic!("Expected Text node");
            };

        if let DialogueNode::Text { text, .. } = deserialized.get_node(NodeId(1)).unwrap() {
            assert_eq!(text, &original_text);
        } else {
            panic!("Node type mismatch");
        }

        // Verify connections are preserved
        let orig_connections = original.get_connected_nodes(NodeId(1));
        let de_connections = deserialized.get_connected_nodes(NodeId(1));
        assert_eq!(orig_connections.len(), de_connections.len());
        if !orig_connections.is_empty() && !de_connections.is_empty() {
            assert_eq!(orig_connections[0].0, de_connections[0].0);
        }
    }
}
