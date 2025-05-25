use funkus_dialogue_core::graph::{DialogueNode, NodeId};
use serde::{Deserialize, Serialize};

/// Wrapper around DialogueNode for the editor
/// 
/// This allows us to add editor-specific functionality without modifying core types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorNode {
    #[serde(flatten)]
    pub node: DialogueNode,
    /// Position in the editor
    pub position: (f32, f32),
}

impl EditorNode {
    pub fn new(node: DialogueNode, position: (f32, f32)) -> Self {
        Self { node, position }
    }
    
    pub fn id(&self) -> NodeId {
        match &self.node {
            DialogueNode::Text { id, .. } => *id,
            DialogueNode::Choice { id, .. } => *id,
        }
    }
}