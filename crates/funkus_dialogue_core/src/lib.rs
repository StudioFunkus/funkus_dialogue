//! # Funkus Dialogue
//!
//! A comprehensive dialogue system for creating interactive narratives in games built with the Bevy engine.

// Module declarations
mod asset;
mod error;
mod events;
pub mod graph;
mod runtime;

// Conditionally include the debug module
#[cfg(feature = "debug_ui")]
mod debug;

// Re-exports for public API
pub use asset::DialogueAsset;
#[cfg(feature = "debug_ui")]
pub use debug::DialogueDebugPlugin;
pub use events::{
    AdvanceDialogue, DialogueChoiceMade, DialogueEnded, DialogueNodeActivated, DialogueStarted,
    SelectDialogueChoice, StartDialogue, StopDialogue,
};
pub use graph::{Connection, DialogueGraph, DialogueNode, NodeId};
pub use runtime::{DialogueRunner, DialogueState};

use bevy::prelude::*;

/// Plugin that sets up the dialogue system components, assets, and systems.
pub struct DialoguePlugin;

impl Plugin for DialoguePlugin {
    fn build(&self, app: &mut App) {
        // Register assets
        app.register_type::<graph::NodeId>()
            .register_type::<runtime::DialogueState>()
            .add_plugins(bevy_common_assets::json::JsonAssetPlugin::<
                asset::DialogueAsset,
            >::new(&["dialogue.json"]));

        // Register events
        app.add_event::<events::DialogueStarted>()
            .add_event::<events::DialogueEnded>()
            .add_event::<events::DialogueNodeActivated>()
            .add_event::<events::DialogueChoiceMade>()
            .add_event::<events::AdvanceDialogue>()
            .add_event::<events::SelectDialogueChoice>()
            .add_event::<events::StartDialogue>()
            .add_event::<events::StopDialogue>();

        // Set up dialogue systems
        runtime::setup_dialogue_systems(app);
    }
}

/// Plugin that includes both the dialogue system and debug tools.
#[cfg(feature = "debug_ui")]
pub struct DialogueDebugBundle;

#[cfg(feature = "debug_ui")]
impl Plugin for DialogueDebugBundle {
    fn build(&self, app: &mut App) {
        app.add_plugins(debug::DialogueDebugPlugin);
    }
}
