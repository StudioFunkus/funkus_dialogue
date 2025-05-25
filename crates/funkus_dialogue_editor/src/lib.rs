//! # Funkus Dialogue Editor
//!
//! A visual node-based editor for creating dialogue graphs for the Funkus Dialogue system.

use bevy::prelude::*;
use bevy_egui::EguiPlugin;

mod editor;
mod node_wrapper;
mod viewer;

pub use editor::{DialogueEditorState, DialogueEditorPlugin};

/// Main plugin that adds the dialogue editor to your Bevy application
pub struct FunkusDialogueEditorPlugin;

impl Plugin for FunkusDialogueEditorPlugin {
    fn build(&self, app: &mut App) {
        // Only add EguiPlugin if it hasn't been added yet
        if !app.is_plugin_added::<EguiPlugin>() {
            app.add_plugins(EguiPlugin { enable_multipass_for_primary_context: true });
        }
        
        app.add_plugins(DialogueEditorPlugin);
        
        info!("Dialogue Editor plugin initialized");
    }
}