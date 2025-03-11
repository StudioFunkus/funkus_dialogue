//! # Core dialogue graph structure.
//! 
//! This module defines the `DialogueGraph` struct, which represents a complete dialogue
//! with its nodes, connections, and metadata.

use bevy::prelude::*;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

use super::node::NodeId;
use super::nodes::NodeType;

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
/// - `nodes`: All nodes in the graph, indexed by their ID
/// 
/// # Example
/// 
/// ```rust
/// use funkus_dialogue::graph::{DialogueGraph, NodeId, NodeType, TextNode};
/// 
/// // Create a new dialogue graph
/// let mut graph = DialogueGraph::new(NodeId(1))
///     .with_name("My Dialogue");
///     
/// // Add a text node
/// let text_node = TextNode::new(NodeId(1), "Hello, world!")
///     .with_speaker("Guide");
///     
/// graph.add_node(NodeType::Text(text_node));
/// 
/// // Validate the graph
/// assert!(graph.validate().is_ok());
/// ```
#[derive(Debug, Clone, Reflect)]
pub struct DialogueGraph {
    /// The underlying directed graph
    #[reflect(ignore)]
    graph: DiGraph<NodeType, Option<String>>,
    /// Mapping from our NodeId to petgraph's NodeIndex
    #[reflect(ignore)]
    node_indices: HashMap<NodeId, NodeIndex>,
    /// The starting node ID for this dialogue
    pub start_node: NodeId,
    /// Optional name or identifier for this dialogue
    pub name: Option<String>,
    /// All nodes in the graph, indexed by their ID (kept for serialization)
    #[reflect(ignore)]
    pub nodes: HashMap<NodeId, NodeType>,
}

impl Serialize for DialogueGraph {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // We'll serialize the old structure for compatibility
        #[derive(Serialize)]
        struct SerializableGraph<'a> {
            nodes: &'a HashMap<NodeId, NodeType>,
            start_node: NodeId,
            name: &'a Option<String>,
        }

        let ser_graph = SerializableGraph {
            nodes: &self.nodes,
            start_node: self.start_node,
            name: &self.name,
        };
        
        ser_graph.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DialogueGraph {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Deserialize into the old structure first
        #[derive(Deserialize)]
        struct DeserializableGraph {
            nodes: HashMap<NodeId, NodeType>,
            start_node: NodeId,
            name: Option<String>,
        }

        let de_graph = DeserializableGraph::deserialize(deserializer)?;
        
        // Convert into our new structure
        let mut graph = DialogueGraph::new(de_graph.start_node);
        graph.name = de_graph.name;
        
        // Add all nodes
        for (_id, node) in de_graph.nodes.iter() {
            graph.add_node(node.clone());
        }
        
        // Add all connections
        for (id, node) in de_graph.nodes.iter() {
            for conn in node.as_node().connections() {
                graph.add_edge(*id, conn.target_id, conn.label.clone())
                    .map_err(serde::de::Error::custom)?;
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
            nodes: HashMap::new(),
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
    /// This method adds a node to both the internal petgraph structure and the node map.
    /// 
    /// # Parameters
    /// 
    /// * `node` - The node to add to the graph
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use funkus_dialogue::graph::{DialogueGraph, NodeId, NodeType, TextNode};
    /// 
    /// let mut graph = DialogueGraph::new(NodeId(1));
    /// let text_node = TextNode::new(NodeId(1), "Hello, world!");
    /// 
    /// graph.add_node(NodeType::Text(text_node));
    /// ```
    pub fn add_node(&mut self, node: NodeType) {
        let id = node.id();
        let index = self.graph.add_node(node.clone());
        self.node_indices.insert(id, index);
        self.nodes.insert(id, node);
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
    /// use funkus_dialogue::graph::{DialogueGraph, NodeId, NodeType, TextNode};
    /// 
    /// let text_node = TextNode::new(NodeId(1), "Hello, world!");
    /// 
    /// let graph = DialogueGraph::new(NodeId(1))
    ///     .with_node(NodeType::Text(text_node));
    /// ```
    pub fn with_node(mut self, node: NodeType) -> Self {
        self.add_node(node);
        self
    }
    
    /// Add an edge between nodes
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
    /// use funkus_dialogue::graph::{DialogueGraph, NodeId, NodeType, TextNode};
    /// 
    /// let mut graph = DialogueGraph::new(NodeId(1));
    /// 
    /// // Add two nodes
    /// graph.add_node(NodeType::Text(TextNode::new(NodeId(1), "First node")));
    /// graph.add_node(NodeType::Text(TextNode::new(NodeId(2), "Second node")));
    /// 
    /// // Connect them
    /// let result = graph.add_edge(NodeId(1), NodeId(2), Some("Next".to_string()));
    /// assert!(result.is_ok());
    /// ```
    pub fn add_edge(&mut self, 
                  from_id: NodeId, 
                  to_id: NodeId, 
                  label: Option<String>) -> Result<(), String> {
        let from_index = self.node_indices.get(&from_id)
            .ok_or_else(|| format!("Source node {:?} not found", from_id))?;
        let to_index = self.node_indices.get(&to_id)
            .ok_or_else(|| format!("Target node {:?} not found", to_id))?;
            
        self.graph.add_edge(*from_index, *to_index, label);
        Ok(())
    }
    
    /// Gets a node by its ID.
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
    /// use funkus_dialogue::graph::{DialogueGraph, NodeId, NodeType, TextNode};
    /// 
    /// let mut graph = DialogueGraph::new(NodeId(1));
    /// graph.add_node(NodeType::Text(TextNode::new(NodeId(1), "Hello")));
    /// 
    /// let node = graph.get_node(NodeId(1));
    /// assert!(node.is_some());
    /// 
    /// let missing_node = graph.get_node(NodeId(99));
    /// assert!(missing_node.is_none());
    /// ```
    pub fn get_node(&self, id: NodeId) -> Option<&NodeType> {
        self.nodes.get(&id)
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
    /// use funkus_dialogue::graph::{DialogueGraph, NodeId, NodeType, TextNode};
    /// 
    /// let mut graph = DialogueGraph::new(NodeId(1));
    /// graph.add_node(NodeType::Text(TextNode::new(NodeId(1), "Start node")));
    /// 
    /// let start_node = graph.get_start_node();
    /// assert!(start_node.is_some());
    /// ```
    pub fn get_start_node(&self) -> Option<&NodeType> {
        self.get_node(self.start_node)
    }
    
    /// Validates the graph structure.
    /// 
    /// This checks for issues like:
    /// - Missing referenced nodes
    /// - Unreachable nodes
    /// - Cycles (although these are allowed in some cases)
    pub fn validate(&self) -> Result<(), String> {
        // For now, just check that all referenced nodes exist
        for node in self.nodes.values() {
            for conn in node.as_node().connections() {
                if !self.nodes.contains_key(&conn.target_id) {
                    return Err(format!(
                        "Node {:?} references non-existent node {:?}",
                        node.id(),
                        conn.target_id
                    ));
                }
            }
        }
        
        // Check that the start node exists
        if !self.nodes.contains_key(&self.start_node) {
            return Err(format!("Start node {:?} does not exist", self.start_node));
        }
        
        // Check for unreachable nodes using petgraph's algorithms
        if let Some(&start_index) = self.node_indices.get(&self.start_node) {
            // Using Petgraph's reachability analysis
            for (node_id, &node_idx) in &self.node_indices {
                if *node_id != self.start_node {
                    let reachable = petgraph::algo::has_path_connecting(&self.graph, start_index, node_idx, None);
                    if !reachable {
                        return Err(format!("Node {:?} is unreachable from start node", node_id));
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Get all nodes connected to the given node
    pub fn get_connected_nodes(&self, id: NodeId) -> Vec<(NodeId, Option<String>)> {
        if let Some(&node_idx) = self.node_indices.get(&id) {
            let edges = self.graph.edges_directed(node_idx, petgraph::Direction::Outgoing);
            edges.filter_map(|edge| {
                let target_idx = edge.target();
                // Find NodeId for this target
                let target_id = self.node_indices.iter()
                    .find_map(|(id, &idx)| if idx == target_idx { Some(*id) } else { None })?;
                    
                Some((target_id, edge.weight().clone()))
            }).collect()
        } else {
            Vec::new()
        }
    }
}