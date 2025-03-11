//! # Node type implementations.
//! 
//! This module contains implementations of various dialogue node types.
//! 
//! ## Node Types
//! 
//! The dialogue system supports these node types:
//! 
//! - **Text Nodes**: Display narrative text with speaker information
//! - **Choice Nodes**: Present options to the player
//! 
//! Additional node types planned for future versions:
//! 
//! - **Condition Nodes**: Branch dialogue based on game state
//! - **Action Nodes**: Trigger events or modify variables
//! - **Jump Nodes**: Move to other parts of the dialogue

mod text_node;
mod choice_node;

pub use text_node::*;
pub use choice_node::*;

use bevy::prelude::*;
use serde::{Serialize, Deserialize};

use super::node::{DialogueNode, NodeId};

/// Enum containing all supported node types.
/// 
/// NodeType is a wrapper enum that allows different node implementations
/// to be treated uniformly within the dialogue system. It implements common
/// methods that delegate to the specific node types inside.
/// 
/// # Variants
/// 
/// * `Text` - Node that displays text from a speaker
/// * `Choice` - Node that presents choices to the player
/// 
/// # Example
/// 
/// ```rust
/// use funkus_dialogue::graph::{NodeType, TextNode, NodeId};
/// 
/// // Create a text node
/// let text_node = TextNode::new(NodeId(1), "Hello, world!");
/// 
/// // Wrap it in the NodeType enum
/// let node_type = NodeType::Text(text_node);
/// 
/// // Get the ID using the common interface
/// assert_eq!(node_type.id(), NodeId(1));
/// ```
#[derive(Debug, Clone, Reflect, Serialize, Deserialize)]
#[serde(crate = "serde")]
pub enum NodeType {
    /// Node that displays text from a speaker
    Text(TextNode),
    /// Node that presents choices to the player
    Choice(ChoiceNode),
}

impl NodeType {
    /// Get a reference to the underlying node.
    /// 
    /// This method provides access to the DialogueNode trait implementation
    /// for the specific node type.
    /// 
    /// # Returns
    /// 
    /// A reference to the underlying node implementing DialogueNode
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use funkus_dialogue::graph::{NodeType, TextNode, NodeId, DialogueNode};
    /// 
    /// let text_node = TextNode::new(NodeId(1), "Hello, world!");
    /// let node_type = NodeType::Text(text_node);
    /// 
    /// let node = node_type.as_node();
    /// assert_eq!(node.id(), NodeId(1));
    /// ```
    pub fn as_node(&self) -> &dyn DialogueNode {
        match self {
            NodeType::Text(node) => node,
            NodeType::Choice(node) => node,
        }
    }
    
    /// Get the ID of this node.
    /// 
    /// This is a convenience method that delegates to the underlying node's
    /// implementation of `id()`.
    /// 
    /// # Returns
    /// 
    /// The NodeId of this node
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use funkus_dialogue::graph::{NodeType, TextNode, NodeId};
    /// 
    /// let text_node = TextNode::new(NodeId(1), "Hello, world!");
    /// let node_type = NodeType::Text(text_node);
    /// 
    /// assert_eq!(node_type.id(), NodeId(1));
    /// ```
    pub fn id(&self) -> NodeId {
        self.as_node().id()
    }
}