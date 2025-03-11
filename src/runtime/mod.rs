//! # Runtime dialogue processing.
//! 
//! This module provides the components and systems for running dialogues at runtime.
//! 
//! ## Overview
//! 
//! The runtime module is responsible for:
//! 
//! - Processing dialogue graphs during gameplay
//! - Managing dialogue state (current node, player choices)
//! - Handling dialogue events (advancement, selection)
//! - Transitioning between dialogue nodes
//! 
//! ## Key Components
//! 
//! - [`DialogueRunner`]: Component that processes and manages a dialogue
//! - [`DialogueState`]: Enum describing the current state of a dialogue
//! - Runtime systems for dialogue processing
//! 
//! ## Usage Example
//! 
//! ```rust
//! use bevy::prelude::*;
//! use funkus_dialogue::*;
//! 
//! fn setup(
//!     mut commands: Commands,
//!     asset_server: Res<AssetServer>,
//!     mut start_events: EventWriter<StartDialogue>,
//! ) {
//!     // Create an entity with a DialogueRunner
//!     let entity = commands.spawn((
//!         Name::new("NPC Dialogue"),
//!         DialogueRunner::default(),
//!     )).id();
//!     
//!     // Load a dialogue asset
//!     let dialogue_handle = asset_server.load("dialogues/npc.dialogue.json");
//!     
//!     // Start the dialogue
//!     start_events.send(StartDialogue {
//!         entity,
//!         dialogue_handle,
//!     });
//! }
//! ```

mod dialogue_runner;
mod systems;

pub use dialogue_runner::*;
pub use systems::*;