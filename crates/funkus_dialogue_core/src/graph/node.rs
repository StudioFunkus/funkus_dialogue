//! # Core node types and traits.
//!
//! This module defines the core types and traits for dialogue nodes.

use bevy::prelude::*;
use petgraph::stable_graph::NodeIndex as StableNodeIndex;
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
/// use funkus_dialogue_core::graph::NodeId;
///
/// let id = NodeId::from_raw(1);
/// assert_eq!(id.raw(), 1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize)]
#[serde(crate = "serde")]
pub struct NodeId(u32);

impl NodeId {
    /// Internal helper to create a NodeId from a petgraph index.
    pub(crate) fn from_index(index: StableNodeIndex) -> Self {
        Self(index.index() as u32 + 1)
    }

    /// Internal helper to convert a NodeId into a petgraph index.
    pub(crate) fn into_index(self) -> StableNodeIndex {
        debug_assert!(self.0 > 0, "NodeId 0 is invalid");
        StableNodeIndex::new((self.0 - 1) as usize)
    }

    /// Exposes the raw numeric value backing this identifier (1-based).
    ///
    /// This remains available for serialization and tooling, but typical
    /// gameplay code should treat `NodeId` as an opaque handle that comes
    /// from `DialogueGraph::add_node`.
    pub fn raw(self) -> u32 {
        self.0
    }

    /// Creates a NodeId from a raw numeric value (1-based).
    ///
    /// Prefer receiving `NodeId` values from the dialogue graph APIs rather
    /// than constructing them manually. This constructor exists primarily
    /// for asset pipelines and migration support.
    pub fn from_raw(id: u32) -> Self {
        Self(id)
    }
}

/// Connection from one node to another.
///
/// This is a convenience struct for tooling and serialization use-cases.
/// The core graph stores edge data using [`ConnectionData`], so most runtime
/// code should work with `ConnectionData` rather than this struct.
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
/// use funkus_dialogue_core::graph::{Connection, NodeId};
///
/// let connection = Connection {
///     target_id: NodeId::from_raw(2),
///     label: Some("Go to the castle".to_string()),
///     choice_key: Some("castle".to_string()),
/// };
///
/// assert_eq!(connection.label.as_deref(), Some("Go to the castle"));
/// ```
#[derive(Debug, Clone, Reflect, Serialize, Deserialize)]
#[serde(crate = "serde")]
pub struct Connection {
    /// The ID of the target node.
    pub target_id: NodeId,
    /// Optional label for this connection.
    pub label: Option<String>,
    /// Optional stable semantic key for this option.
    pub choice_key: Option<String>,
}

/// Data stored on connections between dialogue nodes.
///
/// This struct represents the properties of a connection between two nodes
/// in the dialogue graph. It's stored on the edges of the underlying graph.
#[derive(Debug, Clone, Reflect, Serialize, Deserialize)]
#[serde(crate = "serde")]
pub struct ConnectionData {
    /// Optional label for this connection (used as choice text for choice nodes)
    pub label: Option<String>,
    /// Optional stable semantic key for this connection.
    ///
    /// This stays stable even if labels are renamed or options are reordered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub choice_key: Option<String>,
    /// Optional explicit ordering for this connection among siblings.
    ///
    /// Lower values are shown/processed first. This is assigned automatically
    /// when a connection is created unless explicitly set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<u32>,
}

impl ConnectionData {
    /// Creates a new connection with an optional label
    pub fn new(label: Option<String>) -> Self {
        Self {
            label,
            choice_key: None,
            order: None,
        }
    }

    /// Assigns an explicit ordering value to the connection.
    #[must_use]
    pub fn with_order(mut self, order: u32) -> Self {
        self.order = Some(order);
        self
    }

    /// Assigns a stable semantic key to the connection.
    #[must_use]
    pub fn with_choice_key(mut self, choice_key: impl Into<String>) -> Self {
        self.choice_key = Some(choice_key.into());
        self
    }
}

/// Trait that all dialogue node types must implement.
///
/// The DialogueElement trait defines the common interface that all node types
/// must provide. This allows the dialogue system to work with different node types
/// in a uniform way.
///
/// # Methods
///
/// * `display_name()` - Returns a human-readable name for debugging and UI purposes
///
/// # Example Implementation
///
/// ```rust
/// use funkus_dialogue_core::graph::{DialogueElement, DialogueNode};
///
/// enum MyDialogueNode {
///     Simple {
///         text: String
///     }
/// }
///
/// impl DialogueElement for MyDialogueNode {
///     fn display_name(&self) -> String {
///         match self {
///             MyDialogueNode::Simple { text, .. } => text.clone()
///         }
///     }
/// }
/// ```
pub trait DialogueElement: Send + Sync + 'static {
    /// Returns a display name for debugging and editor purposes.
    fn display_name(&self) -> String;
}
