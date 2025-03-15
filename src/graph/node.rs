//! # Core node types and traits.
//!
//! This module defines the core types and traits for dialogue nodes.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

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
/// The DialogueElement trait defines the common interface that all node types
/// must provide. This allows the dialogue system to work with different node types
/// in a uniform way.
///
/// # Methods
///
/// * `id()` - Returns the unique ID of this node
/// * `connections()` - Returns a reference to the list of connections from this node to other nodes
/// * `display_name()` - Returns a human-readable name for debugging and UI purposes
///
/// # Example Implementation
///
/// ```rust
/// use funkus_dialogue::graph::{DialogueNode, NodeId, Connection};
///
/// enum MyDialogueNode {
///     Simple {
///         id: NodeId,
///         connections: Vec<Connection>,
///         text: String
///     }
/// }
///
/// impl DialogueElement for MyDialogueNode {
///     fn id(&self) -> NodeId {
///         match self {
///             MyDialogueNode::Simple { id, .. } => *id
///         }
///     }
///     
///     fn connections(&self) -> &[Connection] {
///         match self {
///             MyDialogueNode::Simple { connections, .. } => connections
///         }
///     }
///     
///     fn display_name(&self) -> String {
///         match self {
///             MyDialogueNode::Simple { text, .. } => text.clone()
///         }
///     }
/// }
/// ```
pub trait DialogueElement: Send + Sync + 'static {
    /// Returns the unique ID of this node.
    fn id(&self) -> NodeId;

    /// Returns a reference to the list of connections from this node to other nodes.
    fn connections(&self) -> &[Connection];

    /// Returns a display name for debugging and editor purposes.
    fn display_name(&self) -> String;
}
