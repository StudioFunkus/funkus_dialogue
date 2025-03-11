//! # Text node implementation.
//! 
//! This module defines the TextNode type, which represents a node that displays
//! narrative text from a speaker in a dialogue.

use bevy::prelude::*;
use serde::{Serialize, Deserialize};
use crate::graph::node::{BaseNode, Connection, DialogueNode, NodeId};

/// A node that displays text from a speaker.
/// 
/// TextNode is one of the most basic and common node types in the dialogue system.
/// It represents a single piece of dialogue text spoken by a character (or narrator).
/// 
/// When a dialogue reaches a text node, it displays the text content, potentially
/// with speaker information and a portrait, and then waits for the player to advance
/// to the next node. Text nodes typically have zero or one outgoing connections.
/// 
/// # Fields
/// 
/// * `base` - Common node data (ID and connections)
/// * `text` - The text content to display
/// * `speaker` - The name of the speaker (optional)
/// * `portrait` - Optional portrait or avatar identifier for the speaker
/// 
/// # Example
/// 
/// ```rust
/// use funkus_dialogue::graph::{TextNode, NodeId};
/// 
/// // Create a simple text node
/// let node = TextNode::new(NodeId(1), "Hello there!")
///     .with_speaker("Guide")
///     .with_next(NodeId(2));
/// ```
#[derive(Debug, Clone, Reflect, Serialize, Deserialize)]
#[serde(crate = "serde")]
pub struct TextNode {
    /// Common node data
    pub base: BaseNode,
    /// The text content to display
    pub text: String,
    /// The name of the speaker (optional)
    pub speaker: Option<String>,
    /// Optional portrait or avatar identifier for the speaker
    pub portrait: Option<String>,
}

impl TextNode {
    /// Creates a new text node with the given ID and text.
    /// 
    /// # Parameters
    /// 
    /// * `id` - Unique identifier for this node
    /// * `text` - The dialogue text content to display
    /// 
    /// # Returns
    /// 
    /// A new TextNode with the specified ID and text
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use funkus_dialogue::graph::{TextNode, NodeId};
    /// 
    /// let node = TextNode::new(NodeId(1), "Welcome to our village, traveler.");
    /// ```
    pub fn new(id: NodeId, text: impl Into<String>) -> Self {
        Self {
            base: BaseNode::new(id),
            text: text.into(),
            speaker: None,
            portrait: None,
        }
    }
    
    /// Sets the speaker for this node.
    /// 
    /// # Parameters
    /// 
    /// * `speaker` - The name of the character speaking this dialogue
    /// 
    /// # Returns
    /// 
    /// The TextNode with the speaker set
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use funkus_dialogue::graph::{TextNode, NodeId};
    /// 
    /// let node = TextNode::new(NodeId(1), "Have you brought the artifacts?")
    ///     .with_speaker("Village Elder");
    /// ```
    pub fn with_speaker(mut self, speaker: impl Into<String>) -> Self {
        self.speaker = Some(speaker.into());
        self
    }
    
    /// Sets the portrait for this node.
    /// 
    /// # Parameters
    /// 
    /// * `portrait` - Identifier for the portrait/avatar to display
    /// 
    /// # Returns
    /// 
    /// The TextNode with the portrait set
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use funkus_dialogue::graph::{TextNode, NodeId};
    /// 
    /// let node = TextNode::new(NodeId(1), "I sense danger ahead...")
    ///     .with_speaker("Wizard")
    ///     .with_portrait("wizard_concerned");
    /// ```
    pub fn with_portrait(mut self, portrait: impl Into<String>) -> Self {
        self.portrait = Some(portrait.into());
        self
    }
    
    /// Adds a connection to the next node.
    /// 
    /// Text nodes typically have only one outgoing connection to the next
    /// dialogue node. This method is a convenience for setting up that connection.
    /// 
    /// # Parameters
    /// 
    /// * `next_id` - The ID of the next node in the dialogue sequence
    /// 
    /// # Returns
    /// 
    /// The TextNode with the connection added
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use funkus_dialogue::graph::{TextNode, NodeId};
    /// 
    /// let node = TextNode::new(NodeId(1), "What brings you to these parts?")
    ///     .with_speaker("Innkeeper")
    ///     .with_next(NodeId(2));
    /// ```
    pub fn with_next(mut self, next_id: NodeId) -> Self {
        self.base.add_connection(next_id, None);
        self
    }
}

impl DialogueNode for TextNode {
    fn id(&self) -> NodeId {
        self.base.id
    }
    
    fn connections(&self) -> Vec<Connection> {
        self.base.connections.clone()
    }
    
    fn display_name(&self) -> String {
        if let Some(ref speaker) = self.speaker {
            format!("{}: {}", speaker, self.text)
        } else {
            self.text.clone()
        }
    }
}