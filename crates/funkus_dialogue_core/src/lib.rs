//! # Funkus Dialogue
//!
//! A comprehensive dialogue system for creating interactive narratives in games built with the Bevy engine.
//!
//! ## Overview
//!
//! Funkus Dialogue provides a robust framework for implementing interactive dialogues in Bevy games.
//! It handles the full lifecycle of dialogues from asset definition and loading, through runtime processing,
//! to presentation and interaction with the player.
//!
//! ## Core Features
//!
//! - **Asset System**: Define dialogues in JSON format with a flexible node-based structure
//! - **Runtime Engine**: Process dialogues during gameplay, handling player choices and state transitions
//! - **Node Types**: Support for text, choice, and other specialized node types
//! - **Event System**: *Coming soon* - Type-safe events for integrating dialogues with game systems
//! - **Debug Tools**: Built-in debugging utilities for dialogue development
//! - **Editor**: *Coming soon* - A visual editor for creating and editing dialogues
//!
//! ## Basic Usage
//!
//! ```rust
//! use bevy::prelude::*;
//! use funkus_dialogue_core::DialoguePlugin;
//!
//! fn main() {
//!     App::new()
//!         .add_plugins((DefaultPlugins, DialoguePlugin))
//!         .add_systems(Startup, setup_dialogue)
//!         .run();
//! }
//!
//! fn setup_dialogue(
//!     mut commands: Commands,
//!     asset_server: Res<AssetServer>,
//!     mut start_events: EventWriter<funkus_dialogue::StartDialogue>,
//! ) {
//!     // Create an entity to run the dialogue
//!     let entity = commands.spawn((
//!         Name::new("Character Dialogue"),
//!         funkus_dialogue::DialogueRunner::default(),
//!     )).id();
//!     
//!     // Load a dialogue asset
//!     let dialogue_handle = asset_server.load("dialogues/example.dialogue.json");
//!     
//!     // Start the dialogue
//!     start_events.write(funkus_dialogue::StartDialogue {
//!         entity,
//!         dialogue_handle,
//!     });
//! }
//! ```
//!
//! ## Architecture
//!
//! The system follows a layered architecture:
//!
//! 1. **Asset Layer**: Defines dialogue data structures and handles loading from JSON files
//!    into Bevy's asset system. Handles serialization and deserialization of dialogue data.
//! 2. **Graph Layer**: Provides the core graph representation of dialogues, including nodes and connections.
//!    Uses petgraph internally for efficient graph operations while exposing a dialogue-specific API.
//! 3. **Runtime Layer**: Processes dialogues during gameplay, managing state transitions,
//!    handling player choices, and controlling the flow between nodes.
//! 4. **Event Layer**: Connects dialogues with game systems through a bidirectional event system.
//!    Allows game systems to control dialogues and receive notifications about dialogue state changes.
//! 5. **UI Layer**: Handles presentation and player interaction (provided separately or
//!    implemented by the game using the dialogue events).
//!
//! ## Examples
//!
//! For more detailed examples, see the examples directory in the repository:
//!
//! - `simple_dialogue.rs`: A basic dialogue with text and choices
//! - *More examples coming soon*

use bevy::prelude::*;

// Module declarations
mod asset;
mod error;
mod events;
pub mod graph;
mod runtime;


// Re-exports for public API
pub use asset::DialogueAsset;
pub use events::{
    AdvanceDialogue, DialogueChoiceMade, DialogueEnded, DialogueNodeActivated, DialogueStarted,
    SelectDialogueChoice, StartDialogue, StopDialogue,
};
pub use graph::{DialogueGraph, DialogueNode, NodeId};
pub use runtime::{DialogueRunner, DialogueState};

/// Plugin that sets up the dialogue system components, assets, and systems.
///
/// This plugin handles the registration of:
///
/// - Custom assets (DialogueAsset)
/// - Events for dialogue interaction
/// - Systems for processing dialogues
/// - Runtime components
///
/// Add this plugin to your Bevy app to enable the core dialogue functionality.
///
/// # Example
///
/// ```rust
/// use bevy::prelude::*;
/// use funkus_dialogue_core::DialoguePlugin;
///
/// fn main() {
///     App::new()
///         .add_plugins((DefaultPlugins, DialoguePlugin))
///         .run();
/// }
/// ```
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
