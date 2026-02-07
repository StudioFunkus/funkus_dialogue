//! # Node type implementations.
//!
//! This module contains the concrete dialogue node variants that can be
//! inserted into the dialogue graph.
//!
//! ## Node Types
//!
//! The dialogue system currently supports:
//! - **Text Nodes**: Display narrative text with optional speaker metadata
//! - **Choice Nodes**: Present options to the player with an optional prompt
//! - **Effect Nodes**: Apply registry-backed resource changes
//! - **Message Nodes**: Dispatch registered Bevy messages with typed parameters

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::node::DialogueElement;
use crate::registry::{DialogueEffect, DialogueMessageCall};

/// All supported dialogue node variants.
///
/// Nodes focus purely on the data required to render a piece of dialogue; the
/// [`DialogueGraph`](crate::graph::DialogueGraph) is responsible for assigning
/// identifiers and managing connections between them.
///
/// ```rust
/// use funkus_dialogue_core::graph::{ConnectionData, DialogueGraph, DialogueNode};
///
/// let mut graph = DialogueGraph::new();
/// let start = graph.add_node(DialogueNode::text("Hello!"));
/// let reply = graph
///     .add_node(DialogueNode::choice().with_prompt("How do you respond?").unwrap());
///
/// graph
///     .connect(start, reply, ConnectionData::new(None))
///     .unwrap();
/// graph.set_start_node(start).unwrap();
/// ```
#[derive(Debug, Clone, Reflect, Serialize, Deserialize)]
#[serde(crate = "serde")]
pub enum DialogueNode {
    /// Narrative text with optional speaker metadata.
    Text {
        /// Text to present to the player.
        text: String,
        /// Optional speaker name for UI.
        speaker: Option<String>,
        /// Optional portrait or avatar identifier.
        portrait: Option<String>,
    },
    /// A branching point that lets the player choose the next node.
    Choice {
        /// Optional prompt displayed above the choice list.
        prompt: Option<String>,
        /// Optional speaker name for the prompt.
        speaker: Option<String>,
        /// Optional portrait or avatar identifier.
        portrait: Option<String>,
    },
    /// Performs a data-driven effect (resource mutation) and advances immediately.
    Effect {
        /// Effect to apply when the node is activated.
        effect: DialogueEffect,
    },
    /// Dispatches a registered Bevy message and advances immediately.
    Message {
        /// Message payload to send when the node is activated.
        message: DialogueMessageCall,
    },
}

impl DialogueNode {
    /// Builds a new text node with the provided dialogue line.
    pub fn text(text: impl Into<String>) -> Self {
        DialogueNode::Text {
            text: text.into(),
            speaker: None,
            portrait: None,
        }
    }

    /// Builds a new choice node without a prompt.
    pub fn choice() -> Self {
        DialogueNode::Choice {
            prompt: None,
            speaker: None,
            portrait: None,
        }
    }

    /// Builds a new effect node with the provided effect.
    pub fn effect(effect: DialogueEffect) -> Self {
        DialogueNode::Effect { effect }
    }

    /// Builds a new message node with the provided message call payload.
    pub fn message(message: DialogueMessageCall) -> Self {
        DialogueNode::Message { message }
    }

    /// Applies a speaker name to the node.
    pub fn set_speaker(&mut self, speaker: impl Into<String>) {
        match self {
            DialogueNode::Text { speaker: s, .. } | DialogueNode::Choice { speaker: s, .. } => {
                *s = Some(speaker.into());
            }
            DialogueNode::Effect { .. } | DialogueNode::Message { .. } => {}
        }
    }

    /// Removes the speaker metadata from the node.
    pub fn clear_speaker(&mut self) {
        match self {
            DialogueNode::Text { speaker, .. } | DialogueNode::Choice { speaker, .. } => {
                *speaker = None;
            }
            DialogueNode::Effect { .. } | DialogueNode::Message { .. } => {}
        }
    }

    /// Applies a portrait identifier to the node.
    pub fn set_portrait(&mut self, portrait: impl Into<String>) {
        match self {
            DialogueNode::Text { portrait: p, .. } | DialogueNode::Choice { portrait: p, .. } => {
                *p = Some(portrait.into());
            }
            DialogueNode::Effect { .. } | DialogueNode::Message { .. } => {}
        }
    }

    /// Removes the portrait metadata from the node.
    pub fn clear_portrait(&mut self) {
        match self {
            DialogueNode::Text { portrait, .. } | DialogueNode::Choice { portrait, .. } => {
                *portrait = None;
            }
            DialogueNode::Effect { .. } | DialogueNode::Message { .. } => {}
        }
    }

    /// Sets the prompt for a choice node.
    pub fn set_prompt(&mut self, prompt: impl Into<String>) -> Result<(), &'static str> {
        match self {
            DialogueNode::Choice { prompt: p, .. } => {
                *p = Some(prompt.into());
                Ok(())
            }
            _ => Err("Can only set prompt on a Choice node"),
        }
    }

    /// Builder-style helper that assigns a speaker and returns the modified node.
    ///
    /// # Example
    ///
    /// ```rust
    /// use funkus_dialogue_core::graph::DialogueNode;
    ///
    /// let node = DialogueNode::text("Hello there!")
    ///     .with_speaker("Guide");
    /// ```
    pub fn with_speaker(mut self, speaker: impl Into<String>) -> Self {
        self.set_speaker(speaker);
        self
    }

    /// Builder-style helper that assigns a portrait identifier.
    ///
    /// # Example
    ///
    /// ```rust
    /// use funkus_dialogue_core::graph::DialogueNode;
    ///
    /// let node = DialogueNode::text("Hello there!")
    ///     .with_portrait("guide_happy");
    /// ```
    pub fn with_portrait(mut self, portrait: impl Into<String>) -> Self {
        self.set_portrait(portrait);
        self
    }

    /// Builder-style helper for attaching a prompt to a choice node.
    ///
    /// # Example
    ///
    /// ```rust
    /// use funkus_dialogue_core::graph::DialogueNode;
    ///
    /// let node = DialogueNode::choice()
    ///     .with_prompt("What do you do next?").unwrap();
    /// ```
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Result<Self, &'static str> {
        self.set_prompt(prompt)?;
        Ok(self)
    }
}

impl DialogueElement for DialogueNode {
    fn display_name(&self) -> String {
        match self {
            DialogueNode::Text { text, speaker, .. } => {
                if let Some(speaker_name) = speaker {
                    format!("{}: {}", speaker_name, text)
                } else {
                    text.clone()
                }
            }
            DialogueNode::Choice {
                prompt, speaker, ..
            } => {
                if let Some(prompt_text) = prompt {
                    if let Some(speaker_name) = speaker {
                        format!("{}: {} [Choice]", speaker_name, prompt_text)
                    } else {
                        format!("{} [Choice]", prompt_text)
                    }
                } else {
                    "Choice".to_string()
                }
            }
            DialogueNode::Effect { effect } => {
                format!("Effect: {}", effect.key)
            }
            DialogueNode::Message { message } => {
                format!("Message: {}", message.key)
            }
        }
    }
}
