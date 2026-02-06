//! # DialogueAsset Definition
//!
//! This module defines the core asset type for dialogue data.

use crate::asset::DialogueEditorMetadata;
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
/// dialogues to be defined in JSON or RON files. The core plugin registers JSON
/// assets by default; register the RON asset plugin if you want to load `.ron`.
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
/// ```rust,ignore
/// use bevy::prelude::*;
/// use funkus_dialogue_core::asset::DialogueAsset;
///
/// // Load using Bevy's asset system
/// fn setup(asset_server: Res<AssetServer>) {
///     // Load a dialogue asset
///     let dialogue_handle: Handle<DialogueAsset> =
///         asset_server.load("dialogue/example.dialogue.json");
///
///     // The asset can then be accessed through the Assets<DialogueAsset> resource
///     // once it has finished loading
///     let _unused = dialogue_handle;
/// }
/// ```
#[derive(Asset, Debug, Clone, Reflect, Serialize, Deserialize)]
#[serde(crate = "serde")]
pub struct DialogueAsset {
    /// The dialogue graph containing all nodes and connections
    pub graph: DialogueGraph,
    /// Optional name to identify this dialogue
    pub name: Option<String>,
    /// Optional editor-only metadata (node layout, etc.).
    ///
    /// This is safe to ignore for runtime purposes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editor: Option<DialogueEditorMetadata>,
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
    /// use funkus_dialogue_core::{DialogueAsset, DialogueGraph};
    ///
    /// let graph = DialogueGraph::new().with_name("My Dialogue");
    /// let asset = DialogueAsset::new(graph);
    /// // The name is copied from the graph to the asset
    /// assert_eq!(asset.name, Some("My Dialogue".to_string()));
    /// ```
    pub fn new(graph: DialogueGraph) -> Self {
        let name = graph.name.clone();
        Self {
            graph,
            name,
            editor: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::DialogueNode;

    #[test]
    fn dialogue_asset_round_trips_editor_metadata_json() {
        let mut graph = DialogueGraph::new().with_name("Test Dialogue");
        let id = graph.add_node(DialogueNode::text("Hello"));

        let editor = DialogueEditorMetadata {
            nodes: vec![crate::asset::DialogueEditorNodeMetadata {
                id,
                pos: [12.5, 34.25],
                collapsed: true,
            }],
        };

        let mut asset = DialogueAsset::new(graph);
        asset.editor = Some(editor);

        let json = serde_json::to_string_pretty(&asset).expect("serialize DialogueAsset");
        let decoded: DialogueAsset =
            serde_json::from_str(&json).expect("deserialize DialogueAsset");

        let decoded_editor = decoded.editor.expect("editor metadata should exist");
        assert_eq!(decoded_editor.nodes.len(), 1);
        assert_eq!(decoded_editor.nodes[0].id, id);
        assert_eq!(decoded_editor.nodes[0].pos, [12.5, 34.25]);
        assert!(decoded_editor.nodes[0].collapsed);
    }
}
