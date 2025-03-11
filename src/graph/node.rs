//! # Base node trait and common functionality.
//! 
//! This module defines the core types and traits for dialogue nodes.

use bevy::prelude::*;
use serde::{Serialize, Deserialize};

/// Unique identifier for a node in a dialogue graph.
/// 
/// NodeId is a simple wrapper around a u32 that provides type safety
/// and clarity when handling node identifiers. Using a dedicated type
/// instead of raw integers helps prevent errors and makes the code more
/// self-documenting.
/// 
/// # Example
/// 
/// ```rust
/// use funkus_dialogue::graph::NodeId;
/// 
/// let id = NodeId(1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize)]
#[serde(crate = "serde")]
pub struct NodeId(pub u32);

/// Connection from one node to another.
/// 
/// A Connection represents a directed edge in the dialogue graph,
/// potentially with a label. For choice nodes, the label typically
/// represents the text of the choice option.
/// 
/// # Fields
/// 
/// * `target_id` - The ID of the target node
/// * `label` - Optional label for this connection
/// 
/// # Example
/// 
/// ```rust
/// use funkus_dialogue::graph::{NodeId, Connection};
/// 
/// let connection = Connection {
///     target_id: NodeId(2),
///     label: Some("Go to the castle".to_string()),
/// };
/// ```
#[derive(Debug, Clone, Reflect, Serialize, Deserialize)]
#[serde(crate = "serde")]
pub struct Connection {
    /// The ID of the target node.
    pub target_id: NodeId,
    /// Optional label for this connection.
    pub label: Option<String>,
}

/// Trait that all dialogue node types must implement.
/// 
/// The DialogueNode trait defines the common interface that all node types
/// must provide. This allows the dialogue system to work with different node types
/// in a uniform way.
/// 
/// # Methods
/// 
/// * `id()` - Returns the unique ID of this node
/// * `connections()` - Returns the list of connections from this node to other nodes
/// * `display_name()` - Returns a human-readable name for debugging and UI purposes
/// 
/// # Example Implementation
/// 
/// ```rust
/// use funkus_dialogue::graph::{DialogueNode, NodeId, Connection};
/// 
/// struct CustomNode {
///     id: NodeId,
///     connections: Vec<Connection>,
/// }
/// 
/// impl DialogueNode for CustomNode {
///     fn id(&self) -> NodeId {
///         self.id
///     }
///     
///     fn connections(&self) -> Vec<Connection> {
///         self.connections.clone()
///     }
///     
///     fn display_name(&self) -> String {
///         format!("Custom Node {}", self.id.0)
///     }
/// }
/// ```
pub trait DialogueNode: Send + Sync + 'static {
    /// Returns the unique ID of this node.
    fn id(&self) -> NodeId;
    
    /// Returns the list of connections from this node to other nodes.
    fn connections(&self) -> Vec<Connection>;
    
    /// Returns a display name for debugging and editor purposes.
    fn display_name(&self) -> String;
}

/// Common data shared by all node types.
/// 
/// BaseNode provides a common structure and functionality that can be
/// reused by different node type implementations. It handles the basic
/// properties that all nodes share, such as ID and connections.
/// 
/// # Fields
/// 
/// * `id` - Unique identifier for this node
/// * `connections` - Connections to other nodes
/// 
/// # Example
/// 
/// ```rust
/// use funkus_dialogue::graph::{BaseNode, NodeId};
/// 
/// let mut base = BaseNode::new(NodeId(1));
/// base.add_connection(NodeId(2), Some("Next".to_string()));
/// ```
#[derive(Debug, Clone, Reflect, Serialize, Deserialize)]
#[serde(crate = "serde")]
pub struct BaseNode {
    /// Unique identifier for this node
    pub id: NodeId,
    /// Connections to other nodes
    pub connections: Vec<Connection>,
}

impl BaseNode {
    /// Creates a new base node with the given ID and no connections.
    /// 
    /// # Parameters
    /// 
    /// * `id` - The unique identifier for this node
    /// 
    /// # Returns
    /// 
    /// A new BaseNode instance with the given ID and an empty connections list
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use funkus_dialogue::graph::{BaseNode, NodeId};
    /// 
    /// let base = BaseNode::new(NodeId(1));
    /// assert_eq!(base.id, NodeId(1));
    /// assert!(base.connections.is_empty());
    /// ```
    pub fn new(id: NodeId) -> Self {
        Self {
            id,
            connections: Vec::new(),
        }
    }
    
    /// Adds a connection to another node.
    /// 
    /// # Parameters
    /// 
    /// * `target_id` - The ID of the target node
    /// * `label` - Optional label for this connection
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use funkus_dialogue::graph::{BaseNode, NodeId};
    /// 
    /// let mut base = BaseNode::new(NodeId(1));
    /// base.add_connection(NodeId(2), Some("Go to town".to_string()));
    /// 
    /// assert_eq!(base.connections.len(), 1);
    /// assert_eq!(base.connections[0].target_id, NodeId(2));
    /// assert_eq!(base.connections[0].label, Some("Go to town".to_string()));
    /// ```
    pub fn add_connection(&mut self, target_id: NodeId, label: Option<String>) {
        self.connections.push(Connection {
            target_id,
            label,
        });
    }
}