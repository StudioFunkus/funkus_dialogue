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
//! - **Asset System**: Define dialogues in JSON or RON format with a flexible node-based structure
//! - **Runtime Engine**: Process dialogues during gameplay, handling player choices and state transitions
//! - **Node Types**: Support for text, choice, and other specialized node types
//! - **Event System**: Message-based events for integrating dialogues with game systems
//! - **Debug Tools**: Built-in debugging utilities for dialogue development
//! - **Editor**: Visual editor available via the `funkus_dialogue_editor` crate
//!
//! ## Basic Usage
//!
//! ```rust,ignore
//! use bevy::prelude::*;
//! use funkus_dialogue_core::{DialogueAsset, DialoguePlugin, DialogueRunner, StartDialogue};
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
//!     mut start_events: MessageWriter<StartDialogue>,
//! ) {
//!     // Create an entity to run the dialogue
//!     let entity = commands.spawn((
//!         Name::new("Character Dialogue"),
//!         DialogueRunner::default(),
//!     )).id();
//!
//!     // Load a dialogue asset
//!     let dialogue_handle: Handle<DialogueAsset> =
//!         asset_server.load("dialogue/example.dialogue.json");
//!
//!     // Start the dialogue
//!     start_events.write(StartDialogue {
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
//! 1. **Asset Layer**: Defines dialogue data structures and handles loading from JSON/RON files
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

extern crate self as funkus_dialogue_core;

// Module declarations
mod asset;
mod error;
mod events;
pub mod graph;
mod presentation;
pub mod registry;
mod runtime;

// Conditionally include the debug module
#[cfg(feature = "debug_ui")]
mod debug;

// Re-exports for public API
pub use asset::{DialogueAsset, DialogueEditorMetadata, DialogueEditorNodeMetadata};
#[cfg(feature = "debug_ui")]
pub use debug::DialogueDebugPlugin;
pub use error::{DialogueError, DialogueResult};
pub use events::{
    AdvanceDialogue, DialogueChoiceMade, DialogueEnded, DialogueNodeActivated, DialogueStarted,
    SelectDialogueChoice, StartDialogue, StopDialogue,
};
pub use funkus_dialogue_derive::{DialogueMessage, DialogueResource};
pub use graph::{Connection, ConnectionData, DialogueGraph, DialogueNode, NodeId};
pub use presentation::{
    DialogueChoicePresentation, DialogueChoicePresentationAppExt,
    DialogueChoicePresentationDefinition, DialogueChoicePresentationRegistry,
};
pub use registry::{
    DialogueEffect, DialogueMessage, DialogueMessageCall, DialogueMessageDefinition,
    DialogueMessageError, DialogueMessageField, DialogueMessageParam, DialogueMessageRegistry,
    DialogueMessageRegistryPlugin, DialogueMessageTypeData, DialogueOperation, DialogueRegistry,
    DialogueRegistryPlugin, DialogueResource, DialogueResourceTypeData, DialogueValue,
};
pub use runtime::{DialogueRunner, DialogueState, DialogueSystemSet};

#[doc(hidden)]
pub mod __private {
    pub use inventory;
}

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
/// ```rust,ignore
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
            .register_type::<registry::DialogueEffect>()
            .register_type::<registry::DialogueMessageCall>()
            .register_type::<registry::DialogueValue>()
            .add_plugins(bevy_common_assets::json::JsonAssetPlugin::<
                asset::DialogueAsset,
            >::new(&["dialogue.json"]));

        // Register events
        app.add_message::<events::DialogueStarted>()
            .add_message::<events::DialogueEnded>()
            .add_message::<events::DialogueNodeActivated>()
            .add_message::<events::DialogueChoiceMade>()
            .add_message::<events::AdvanceDialogue>()
            .add_message::<events::SelectDialogueChoice>()
            .add_message::<events::StartDialogue>()
            .add_message::<events::StopDialogue>();

        // Set up dialogue systems
        runtime::setup_dialogue_systems(app);
        app.add_plugins((
            registry::DialogueRegistryPlugin,
            registry::DialogueMessageRegistryPlugin,
        ));
    }
}

/// Plugin that includes the dialogue system debug tools.
///
/// # Example
///
/// ```rust,ignore
/// use bevy::prelude::*;
/// use funkus_dialogue_core::{DialogueDebugBundle, DialoguePlugin};
///
/// fn main() {
///     App::new()
///         .add_plugins((DefaultPlugins, DialoguePlugin, DialogueDebugBundle))
///         .run();
/// }
/// ```
#[cfg(feature = "debug_ui")]
pub struct DialogueDebugBundle;

#[cfg(feature = "debug_ui")]
impl Plugin for DialogueDebugBundle {
    fn build(&self, app: &mut App) {
        app.add_plugins(debug::DialogueDebugPlugin);
    }
}
