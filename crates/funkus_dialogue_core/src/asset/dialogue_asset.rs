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
/// dialogues to be defined in RON files.
///
/// # Example RON Format
///
/// ```ron
/// TODO: Add example
/// ```
///
/// # Loading with Bevy
///
/// ```rust
/// fn setup(asset_server: Res<AssetServer>) {
///     let dialogue_handle = asset_server.load("dialogues/example.dialogue.ron");
/// }
/// ```
#[derive(Asset, Debug, Clone, Reflect, Serialize, Deserialize)]
#[serde(crate = "serde")]
pub struct DialogueAsset {
    /// The dialogue graph containing all nodes and connections
    pub graph: DialogueGraph,
}

impl DialogueAsset {
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
    /// let graph = DialogueGraph::new(NodeId(1)).with_name("My Dialogue");
    /// let asset = DialogueAsset::new(graph);
    /// ```
    pub fn new(graph: DialogueGraph) -> Self {
        Self { graph }
    }
}
