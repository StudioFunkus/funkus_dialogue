//! # DialogueAsset Definition
//!
//! This module defines the core asset type for dialogue data.

use crate::graph::DialogueGraph;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

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
///     "nodes": [
///       {
///         "type": "Text",
///         "id": 1,
///         "text": "Hello there!",
///         "speaker": "Guide",
///         "portrait": null
///       }
///     ],
///     "connections": [
///       {
///         "from": 1,
///         "to": 2,
///         "label": null
///       }
///     ],
///     "start_node": 1,
///     "name": "Example Dialogue"
///   }
/// }
/// ```
///
/// # Loading with Bevy
///
/// ```rust
/// // Load using Bevy's asset system
/// fn setup(asset_server: Res<AssetServer>) {
///     // Load a dialogue asset
///     let dialogue_handle = asset_server.load("dialogues/example.dialogue.json");
///     
///     // The asset can then be accessed through the Assets<DialogueAsset> resource
///     // once it has finished loading
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
    /// This constructor copies the name from the graph's name field and uses it
    /// as the asset name.
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
    /// // The name is copied from the graph to the asset
    /// assert_eq!(asset.name, Some("My Dialogue".to_string()));
    /// ```
    pub fn new(graph: DialogueGraph) -> Self {
        let name = graph.name.clone();
        Self { graph, name }
    }
}
