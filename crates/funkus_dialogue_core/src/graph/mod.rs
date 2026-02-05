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
//! - [`ConnectionData`]: Edge data between nodes, including optional labels
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
//! - Text nodes are expected to have 0 or 1 outgoing connection at runtime
//!
//! ## Ordering
//!
//! Connections carry an explicit `order` field (stored in [`ConnectionData`]).
//! When a connection is created without an order, the graph assigns the next
//! available value. Use this to render or process choices deterministically,
//! and update it in editor tooling when reordering choices.
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
/// use funkus_dialogue_core::graph::{ConnectionData, DialogueGraph, DialogueNode};
///
/// let mut graph = DialogueGraph::new().with_name("Simple Dialogue");
///
/// let start = graph
///     .add_node(DialogueNode::text("Hello there!").with_speaker("Guide"));
/// let choice = graph
///     .add_node(
///         DialogueNode::choice()
///             .with_speaker("Guide")
///             .with_prompt("How would you like to respond?").unwrap()
///     );
///
/// graph
///     .connect(start, choice, ConnectionData::new(None))
///     .unwrap();
/// graph.set_start_node(start).unwrap();
/// ```
mod dialogue_graph;
pub mod node;
mod nodes;

pub use dialogue_graph::*;
pub use node::*;
pub use nodes::*;
