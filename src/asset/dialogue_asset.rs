//! # DialogueAsset Definition
//! 
//! This module defines the core asset type for dialogue data.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use crate::graph::DialogueGraph;

/// Asset type for dialogue data.
/// 
/// `DialogueAsset` represents a complete dialogue that can be loaded from a file.
/// It contains a dialogue graph that defines the structure of the dialogue, including
/// all nodes, connections, and metadata.
/// 
/// # Structure
/// 
/// - `graph`: The dialogue graph containing all nodes and connections
/// - `name`: Optional name to identify this dialogue
/// 
/// # Serialization
/// 
/// This type supports serialization and deserialization through serde, allowing
/// dialogues to be defined in JSON files.
/// 
/// # Example JSON Format
/// 
/// ```json
/// {
///   "graph": {
///     "nodes": {
///       "1": {
///         "Text": {
///           "base": {
///             "id": 1,
///             "connections": [
///               {
///                 "target_id": 2,
///                 "label": null
///               }
///             ]
///           },
///           "text": "Hello there!",
///           "speaker": "Guide",
///           "portrait": null
///         }
///       }
///     },
///     "start_node": 1,
///     "name": "Example Dialogue"
///   },
///   "name": "Example Dialogue"
/// }
/// ```
#[derive(Asset, Debug, Clone, Reflect, Serialize, Deserialize)]
#[serde(crate = "serde")]
pub struct DialogueAsset {
    /// The dialogue graph containing all nodes and connections
    pub graph: DialogueGraph,
    /// Optional name to identify this dialogue
    pub name: Option<String>,
}

impl DialogueAsset {
    /// Creates a new dialogue asset from a dialogue graph.
    /// 
    /// This constructor extracts the name from the graph and sets it as the asset name.
    /// 
    /// # Parameters
    /// 
    /// * `graph` - The dialogue graph to include in this asset
    /// 
    /// # Returns
    /// 
    /// A new DialogueAsset containing the provided graph
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use funkus_dialogue::{DialogueAsset, DialogueGraph, NodeId};
    /// 
    /// let graph = DialogueGraph::new(NodeId(1)).with_name("My Dialogue");
    /// let asset = DialogueAsset::new(graph);
    /// assert_eq!(asset.name, Some("My Dialogue".to_string()));
    /// ```
    pub fn new(graph: DialogueGraph) -> Self {
        let name = graph.name.clone();
        Self {
            graph,
            name,
        }
    }
}