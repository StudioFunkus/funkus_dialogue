//! Editor-only metadata for dialogue assets.
//!
//! `funkus_dialogue` keeps the runtime dialogue semantics (nodes, connections, ordering) inside
//! [`DialogueGraph`]. However, a node editor also needs *tooling state* that should not affect
//! runtime behavior: node positions, collapsed state, viewport settings, etc.
//!
//! This module defines **optional** metadata that may be embedded in [`DialogueAsset`].
//! Games and the runtime should treat this data as best-effort hints and are free to ignore it.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::graph::NodeId;

/// Tooling-only metadata for a dialogue asset.
///
/// This is written by editors to persist layout between sessions. It is intentionally kept
/// separate from the runtime dialogue semantics so that:
///
/// - Runtime behavior does not depend on editor state.
/// - Other editors (or hand-authored JSON) can omit it entirely.
/// - Future editor improvements can extend this data without breaking existing games.
#[derive(Debug, Clone, Default, Reflect, Serialize, Deserialize)]
#[serde(crate = "serde")]
pub struct DialogueEditorMetadata {
    /// Per-node layout data.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<DialogueEditorNodeMetadata>,
}

impl DialogueEditorMetadata {
    /// Returns the metadata entry for a node id, if present.
    #[must_use]
    pub fn node(&self, id: NodeId) -> Option<&DialogueEditorNodeMetadata> {
        self.nodes.iter().find(|entry| entry.id == id)
    }
}

/// Tooling-only layout metadata for a single node.
#[derive(Debug, Clone, Copy, PartialEq, Reflect, Serialize, Deserialize)]
#[serde(crate = "serde")]
pub struct DialogueEditorNodeMetadata {
    /// The node id this metadata applies to.
    pub id: NodeId,
    /// The top-left position of the node in canvas space.
    ///
    /// We intentionally store this as a primitive array to avoid coupling the asset format to
    /// any particular math/UI type.
    pub pos: [f32; 2],
    /// If true, the node should start collapsed in the editor.
    #[serde(default, skip_serializing_if = "is_false")]
    pub collapsed: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}
