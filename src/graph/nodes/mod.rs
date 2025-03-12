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

use bevy::prelude::*;
use serde::{Serialize, Deserialize};

use super::node::{DialogueElement, NodeId, Connection};

/// Enum containing all supported node types.
/// 
/// DialogueNode is the core representation of different node types in the dialogue system.
/// It uses Rust's enum pattern to represent different node variants directly, with each
/// variant containing all the necessary fields for that node type.
/// 
/// # Variants
/// 
/// * `Text` - Node that displays text from a speaker
/// * `Choice` - Node that presents choices to the player
/// 
/// # Example
/// 
/// ```rust
/// use funkus_dialogue::graph::{DialogueNode, NodeId};
/// 
/// // Create a text node
/// let text_node = DialogueNode::text(NodeId(1), "Hello, world!");
/// 
/// // Get the ID using the common interface
/// assert_eq!(text_node.id(), NodeId(1));
/// ```
#[derive(Debug, Clone, Reflect, Serialize, Deserialize)]
#[serde(crate = "serde")]
pub enum DialogueNode {
    /// Node that displays text from a speaker
    Text {
        /// Unique identifier for this node
        id: NodeId,
        /// Connections to other nodes
        connections: Vec<Connection>,
        /// The text content to display
        text: String,
        /// The name of the speaker (optional)
        speaker: Option<String>,
        /// Optional portrait or avatar identifier for the speaker
        portrait: Option<String>,
    },
    /// Node that presents choices to the player
    Choice {
        /// Unique identifier for this node
        id: NodeId,
        /// Connections to other nodes (these are the choices)
        connections: Vec<Connection>,
        /// Optional prompt text to display before the choices
        prompt: Option<String>,
        /// Optional speaker for the prompt
        speaker: Option<String>,
        /// Optional portrait or avatar identifier for the speaker
        portrait: Option<String>,
    },
}

impl DialogueNode {
    /// Creates a new text node with the given ID and text.
    /// 
    /// # Parameters
    /// 
    /// * `id` - Unique identifier for this node
    /// * `text` - The dialogue text content to display
    /// 
    /// # Returns
    /// 
    /// A new Text node with the specified ID and text
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use funkus_dialogue::graph::{DialogueNode, NodeId};
    /// 
    /// let node = DialogueNode::text(NodeId(1), "Welcome to our village, traveler.");
    /// ```
    pub fn text(id: NodeId, text: impl Into<String>) -> Self {
        DialogueNode::Text {
            id,
            connections: Vec::new(),
            text: text.into(),
            speaker: None,
            portrait: None,
        }
    }
    
    /// Creates a new choice node with the given ID.
    /// 
    /// # Parameters
    /// 
    /// * `id` - Unique identifier for this node
    /// 
    /// # Returns
    /// 
    /// A new Choice node with the specified ID
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use funkus_dialogue::graph::{DialogueNode, NodeId};
    /// 
    /// let node = DialogueNode::choice(NodeId(2));
    /// ```
    pub fn choice(id: NodeId) -> Self {
        DialogueNode::Choice {
            id,
            connections: Vec::new(),
            prompt: None,
            speaker: None,
            portrait: None,
        }
    }
    
    /// Adds a connection to another node.
    /// 
    /// This method can be used with any node type to add an outgoing connection.
    /// 
    /// # Parameters
    /// 
    /// * `target_id` - The ID of the target node
    /// * `label` - Optional label for this connection
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use funkus_dialogue::graph::{DialogueNode, NodeId};
    /// 
    /// let mut node = DialogueNode::text(NodeId(1), "Hello world");
    /// node.add_connection(NodeId(2), None);
    /// ```
    pub fn add_connection(&mut self, target_id: NodeId, label: Option<String>) {
        let connections = match self {
            DialogueNode::Text { connections, .. } => connections,
            DialogueNode::Choice { connections, .. } => connections,
        };
        
        connections.push(Connection {
            target_id,
            label,
        });
    }
    
    /// Sets the speaker for this node.
    /// 
    /// This method can be used with any node type to set the speaker.
    /// 
    /// # Parameters
    /// 
    /// * `speaker` - The name of the character speaking
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use funkus_dialogue::graph::{DialogueNode, NodeId};
    /// 
    /// let mut node = DialogueNode::text(NodeId(1), "Hello world");
    /// node.set_speaker("Guide");
    /// ```
    pub fn set_speaker(&mut self, speaker: impl Into<String>) {
        match self {
            DialogueNode::Text { speaker: s, .. } => *s = Some(speaker.into()),
            DialogueNode::Choice { speaker: s, .. } => *s = Some(speaker.into()),
        }
    }
    
    /// Sets the portrait for this node.
    /// 
    /// This method can be used with any node type to set the portrait.
    /// 
    /// # Parameters
    /// 
    /// * `portrait` - Identifier for the portrait/avatar to display
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use funkus_dialogue::graph::{DialogueNode, NodeId};
    /// 
    /// let mut node = DialogueNode::text(NodeId(1), "Hello world");
    /// node.set_portrait("guide_happy");
    /// ```
    pub fn set_portrait(&mut self, portrait: impl Into<String>) {
        match self {
            DialogueNode::Text { portrait: p, .. } => *p = Some(portrait.into()),
            DialogueNode::Choice { portrait: p, .. } => *p = Some(portrait.into()),
        }
    }
    
    /// Sets the text content for a Text node.
    /// 
    /// # Parameters
    /// 
    /// * `text` - The text content to set
    /// 
    /// # Returns
    /// 
    /// Ok(()) if successful, or an error if this is not a Text node
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use funkus_dialogue::graph::{DialogueNode, NodeId};
    /// 
    /// let mut node = DialogueNode::text(NodeId(1), "Hello");
    /// node.set_text("Updated text").unwrap();
    /// ```
    pub fn set_text(&mut self, text: impl Into<String>) -> Result<(), &'static str> {
        match self {
            DialogueNode::Text { text: t, .. } => {
                *t = text.into();
                Ok(())
            }
            _ => Err("Can only set text on a Text node"),
        }
    }
    
    /// Sets the prompt for a Choice node.
    /// 
    /// # Parameters
    /// 
    /// * `prompt` - The prompt text to set
    /// 
    /// # Returns
    /// 
    /// Ok(()) if successful, or an error if this is not a Choice node
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use funkus_dialogue::graph::{DialogueNode, NodeId};
    /// 
    /// let mut node = DialogueNode::choice(NodeId(2));
    /// node.set_prompt("What would you like to do?").unwrap();
    /// ```
    pub fn set_prompt(&mut self, prompt: impl Into<String>) -> Result<(), &'static str> {
        match self {
            DialogueNode::Choice { prompt: p, .. } => {
                *p = Some(prompt.into());
                Ok(())
            }
            _ => Err("Can only set prompt on a Choice node"),
        }
    }
    
    /// Adds a choice option to a Choice node.
    /// 
    /// # Parameters
    /// 
    /// * `text` - The text for this choice option
    /// * `target_id` - The ID of the node to go to if this choice is selected
    /// 
    /// # Returns
    /// 
    /// Ok(()) if successful, or an error if this is not a Choice node
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use funkus_dialogue::graph::{DialogueNode, NodeId};
    /// 
    /// let mut node = DialogueNode::choice(NodeId(2));
    /// node.add_choice("Go to town", NodeId(3)).unwrap();
    /// ```
    pub fn add_choice(&mut self, text: impl Into<String>, target_id: NodeId) -> Result<(), &'static str> {
        match self {
            DialogueNode::Choice { connections, .. } => {
                connections.push(Connection {
                    target_id,
                    label: Some(text.into()),
                });
                Ok(())
            }
            _ => Err("Can only add choices to a Choice node"),
        }
    }

    // Builder pattern methods
    
    /// Builder method to set the speaker.
    /// 
    /// # Parameters
    /// 
    /// * `speaker` - The name of the speaker
    /// 
    /// # Returns
    /// 
    /// The node with the speaker set
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use funkus_dialogue::graph::{DialogueNode, NodeId};
    /// 
    /// let node = DialogueNode::text(NodeId(1), "Hello")
    ///     .with_speaker("Guide");
    /// ```
    pub fn with_speaker(mut self, speaker: impl Into<String>) -> Self {
        self.set_speaker(speaker);
        self
    }
    
    /// Builder method to set the portrait.
    /// 
    /// # Parameters
    /// 
    /// * `portrait` - Identifier for the portrait/avatar
    /// 
    /// # Returns
    /// 
    /// The node with the portrait set
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use funkus_dialogue::graph::{DialogueNode, NodeId};
    /// 
    /// let node = DialogueNode::text(NodeId(1), "Hello")
    ///     .with_portrait("guide_happy");
    /// ```
    pub fn with_portrait(mut self, portrait: impl Into<String>) -> Self {
        self.set_portrait(portrait);
        self
    }
    
    /// Builder method to add a connection to the next node.
    /// 
    /// # Parameters
    /// 
    /// * `target_id` - The ID of the next node
    /// 
    /// # Returns
    /// 
    /// The node with the connection added
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use funkus_dialogue::graph::{DialogueNode, NodeId};
    /// 
    /// let node = DialogueNode::text(NodeId(1), "Hello")
    ///     .with_next(NodeId(2));
    /// ```
    pub fn with_next(mut self, target_id: NodeId) -> Self {
        self.add_connection(target_id, None);
        self
    }
    
    /// Builder method to set the prompt for a Choice node.
    /// 
    /// # Parameters
    /// 
    /// * `prompt` - The prompt text
    /// 
    /// # Returns
    /// 
    /// The node with the prompt set, or an error if not a Choice node
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use funkus_dialogue::graph::{DialogueNode, NodeId};
    /// 
    /// let node = DialogueNode::choice(NodeId(2))
    ///     .with_prompt("What would you like to do?").unwrap();
    /// ```
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Result<Self, &'static str> {
        self.set_prompt(prompt)?;
        Ok(self)
    }
    
    /// Builder method to add a choice to a Choice node.
    /// 
    /// # Parameters
    /// 
    /// * `text` - The text for this choice
    /// * `target_id` - The target node ID if this choice is selected
    /// 
    /// # Returns
    /// 
    /// The node with the choice added, or an error if not a Choice node
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use funkus_dialogue::graph::{DialogueNode, NodeId};
    /// 
    /// let node = DialogueNode::choice(NodeId(2))
    ///     .with_choice("Go to town", NodeId(3)).unwrap();
    /// ```
    pub fn with_choice(mut self, text: impl Into<String>, target_id: NodeId) -> Result<Self, &'static str> {
        self.add_choice(text, target_id)?;
        Ok(self)
    }
}

impl DialogueElement for DialogueNode {
    fn id(&self) -> NodeId {
        match self {
            DialogueNode::Text { id, .. } => *id,
            DialogueNode::Choice { id, .. } => *id,
        }
    }
    
    fn connections(&self) -> &[Connection] {
        match self {
            DialogueNode::Text { connections, .. } => connections,
            DialogueNode::Choice { connections, .. } => connections,
        }
    }
    
    fn display_name(&self) -> String {
        match self {
            DialogueNode::Text { text, speaker, .. } => {
                if let Some(speaker_name) = speaker {
                    format!("{}: {}", speaker_name, text)
                } else {
                    text.clone()
                }
            },
            DialogueNode::Choice { prompt, speaker, .. } => {
                if let Some(prompt_text) = prompt {
                    if let Some(speaker_name) = speaker {
                        format!("{}: {} [Choice]", speaker_name, prompt_text)
                    } else {
                        format!("{} [Choice]", prompt_text)
                    }
                } else {
                    "Choice".to_string()
                }
            },
        }
    }
}