//! # Dialogue Graph System
//! 
//! This module defines the core structures that represent dialogue graphs, nodes, and connections.
//! 
//! ## Overview
//! 
//! The graph system provides:
//! 
//! - A flexible graph structure for representing dialogues using petgraph
//! - Various node types for different dialogue elements
//! - Connection management between nodes
//! - Serialization/deserialization support
//! 
//! ## Key Components
//! 
//! - [`DialogueGraph`]: The main graph structure containing nodes and connections
//! - [`NodeId`]: Unique identifier for nodes in a graph
//! - [`ConnectionId`]: Identifier for connections between nodes
//! - [`NodeType`]: Enum of different node implementations
//! - [`TextNode`]: Node for displaying text from a speaker
//! - [`ChoiceNode`]: Node for presenting choices to the player
//! 
//! ## Graph Structure
//! 
//! Dialogues are represented as directed graphs where:
//! 
//! - Each node represents a specific dialogue element (text, choice, etc.)
//! - Connections between nodes define the possible paths through the dialogue
//! - The graph has a designated start node where dialogues begin
//! - Nodes without outgoing connections represent dialogue endpoints
//! 
//! ## Node Types
//! 
//! The system currently supports these node types:
//! 
//! - **Text Nodes**: Display narrative text with speaker information
//! - **Choice Nodes**: Present options to the player
//! 
//! Additional node types planned for future versions include:
//! 
//! - **Condition Nodes**: Branch dialogue based on game state
//! - **Action Nodes**: Trigger events or modify variables
//! - **Jump Nodes**: Move to other parts of the dialogue
//! 
//! ## Example Usage
//! 
//! ```rust
//! use funkus_dialogue::graph::{DialogueGraph, NodeId, NodeType, TextNode, ChoiceNode};
//! 
//! // Create a new dialogue graph
//! let mut graph = DialogueGraph::new(NodeId(1))
//!     .with_name("Simple Dialogue");
//!     
//! // Add a text node
//! let text_node = TextNode::new(NodeId(1), "Hello there!")
//!     .with_speaker("Guide")
//!     .with_next(NodeId(2));
//!     
//! // Add a choice node
//! let mut choice_node = ChoiceNode::new(NodeId(2))
//!     .with_prompt("How would you like to respond?")
//!     .with_speaker("Guide");
//!     
//! choice_node.add_choice("Nice to meet you!", NodeId(3));
//! choice_node.add_choice("Goodbye.", NodeId(4));
//! 
//! // Add nodes to the graph
//! graph.add_node(NodeType::Text(text_node));
//! graph.add_node(NodeType::Choice(choice_node));
//! ```

pub mod node;
mod nodes;
mod dialogue_graph;

pub use node::*;
pub use nodes::*;
pub use dialogue_graph::*;