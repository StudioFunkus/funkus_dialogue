//! # Funkus Dialogue Editor
//!
//! A visual node-based editor for creating dialogue graphs for the Funkus Dialogue system.

use bevy::prelude::*;

mod editor;
mod node_wrapper;
mod viewer;

pub use editor::{DialogueEditorState, DialogueEditorPlugin};

/// Main plugin that adds the dialogue editor to your Bevy application
pub struct FunkusDialogueEditorPlugin;

impl Plugin for FunkusDialogueEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DialogueEditorPlugin);
        
        info!("Dialogue Editor plugin initialized");
    }
}