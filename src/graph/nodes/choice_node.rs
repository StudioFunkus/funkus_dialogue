//! # Choice node implementation.
//! 
//! This module defines the ChoiceNode type, which represents a node that presents
//! a set of choices to the player in a dialogue.

use bevy::prelude::*;
use serde::{Serialize, Deserialize};
use crate::graph::node::{BaseNode, Connection, DialogueNode, NodeId};

/// A node that presents choices to the player.
/// 
/// ChoiceNode is a key node type that enables branching dialogues. It presents
/// the player with a set of options to choose from, each leading to a different
/// path in the dialogue.
/// 
/// When a dialogue reaches a choice node, it displays the prompt (if any) and
/// the available choices, then waits for the player to select one. Each choice
/// corresponds to an outgoing connection to another node.
/// 
/// # Fields
/// 
/// * `base` - Common node data (ID and connections)
/// * `prompt` - Optional prompt text to display before the choices
/// * `speaker` - Optional speaker for the prompt
/// * `portrait` - Optional portrait or avatar identifier for the speaker
/// 
/// # Example
/// 
/// ```rust
/// use funkus_dialogue::graph::{ChoiceNode, NodeId};
/// 
/// // Create a choice node with options
/// let mut node = ChoiceNode::new(NodeId(2))
///     .with_prompt("How do you respond?")
///     .with_speaker("Merchant");
///     
/// // Add choices
/// node.add_choice("I'll buy it", NodeId(3));
/// node.add_choice("The price is too high", NodeId(4));
/// node.add_choice("Let me think about it", NodeId(5));
/// ```
#[derive(Debug, Clone, Reflect, Serialize, Deserialize)]
#[serde(crate = "serde")]
pub struct ChoiceNode {
    /// Common node data
    pub base: BaseNode,
    /// Optional prompt text to display before the choices
    pub prompt: Option<String>,
    /// Optional speaker for the prompt
    pub speaker: Option<String>,
    /// Optional portrait or avatar identifier for the speaker
    pub portrait: Option<String>,
}

impl ChoiceNode {
    /// Creates a new choice node with the given ID.
    /// 
    /// # Parameters
    /// 
    /// * `id` - Unique identifier for this node
    /// 
    /// # Returns
    /// 
    /// A new ChoiceNode with the specified ID and no choices
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use funkus_dialogue::graph::{ChoiceNode, NodeId};
    /// 
    /// let node = ChoiceNode::new(NodeId(2));
    /// ```
    pub fn new(id: NodeId) -> Self {
        Self {
            base: BaseNode::new(id),
            prompt: None,
            speaker: None,
            portrait: None,
        }
    }
    
    /// Sets the prompt text for this choice node.
    /// 
    /// The prompt is the text displayed before the choice options,
    /// typically a question or statement that the choices are responding to.
    /// 
    /// # Parameters
    /// 
    /// * `prompt` - The prompt text to display
    /// 
    /// # Returns
    /// 
    /// The ChoiceNode with the prompt set
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use funkus_dialogue::graph::{ChoiceNode, NodeId};
    /// 
    /// let node = ChoiceNode::new(NodeId(2))
    ///     .with_prompt("What would you like to do?");
    /// ```
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }
    
    /// Sets the speaker for this node.
    /// 
    /// # Parameters
    /// 
    /// * `speaker` - The name of the character speaking the prompt
    /// 
    /// # Returns
    /// 
    /// The ChoiceNode with the speaker set
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use funkus_dialogue::graph::{ChoiceNode, NodeId};
    /// 
    /// let node = ChoiceNode::new(NodeId(2))
    ///     .with_prompt("Will you help us?")
    ///     .with_speaker("Village Chief");
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
    /// The ChoiceNode with the portrait set
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use funkus_dialogue::graph::{ChoiceNode, NodeId};
    /// 
    /// let node = ChoiceNode::new(NodeId(2))
    ///     .with_prompt("Will you join our quest?")
    ///     .with_speaker("Knight Captain")
    ///     .with_portrait("knight_formal");
    /// ```
    pub fn with_portrait(mut self, portrait: impl Into<String>) -> Self {
        self.portrait = Some(portrait.into());
        self
    }
    
    /// Adds a choice option that leads to the specified node.
    pub fn add_choice(&mut self, text: impl Into<String>, target_id: NodeId) {
        self.base.add_connection(target_id, Some(text.into()));
    }
    
    /// Builder-style method to add a choice option.
    pub fn with_choice(mut self, text: impl Into<String>, target_id: NodeId) -> Self {
        self.add_choice(text, target_id);
        self
    }
}

impl DialogueNode for ChoiceNode {
    fn id(&self) -> NodeId {
        self.base.id
    }
    
    fn connections(&self) -> Vec<Connection> {
        self.base.connections.clone()
    }
    
    fn display_name(&self) -> String {
        if let Some(ref prompt) = self.prompt {
            if let Some(ref speaker) = self.speaker {
                format!("{}: {} [Choice]", speaker, prompt)
            } else {
                format!("{} [Choice]", prompt)
            }
        } else {
            "Choice".to_string()
        }
    }
}