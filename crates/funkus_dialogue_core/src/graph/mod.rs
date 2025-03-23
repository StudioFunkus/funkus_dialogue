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
//! - [`Connection`]: Connection between nodes, potentially with a label
//! - [`DialogueNode`]: Enum of different node implementations
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
/// ## Example Usage
///
/// ```rust
/// use funkus_dialogue::graph::{DialogueGraph, NodeId, DialogueNode};
///
/// // Create a new dialogue graph
/// let mut graph = DialogueGraph::new(NodeId(1))
///     .with_name("Simple Dialogue");
///     
/// // Add a text node
/// let text_node = DialogueNode::text(NodeId(1), "Hello there!")
///     .with_speaker("Guide");
///     
/// // Add a choice node
/// let choice_node = DialogueNode::choice(NodeId(2))
///     .with_speaker("Guide")
///     .with_prompt("How would you like to respond?").unwrap();
///
/// // Add nodes to the graph
/// graph.add_node(text_node);
/// graph.add_node(choice_node);
///
/// // Connect nodes using the graph
/// graph.add_edge(NodeId(1), NodeId(2), None).unwrap();
/// graph.add_edge(NodeId(2), NodeId(3), Some("Nice to meet you!".to_string())).unwrap();
/// graph.add_edge(NodeId(2), NodeId(4), Some("Goodbye.".to_string())).unwrap();
/// ```

mod dialogue_graph;
pub mod node;
mod nodes;

pub use dialogue_graph::*;
pub use node::*;
pub use nodes::*;
