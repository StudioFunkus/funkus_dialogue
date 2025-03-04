//! # Funkus Dialogue
//! 
//! A comprehensive dialogue system for creating interactive narratives in games built with the Bevy engine.
//! 
//! ## Overview
//! 
//! This library provides:
//! 
//! - A dialogue asset system for defining conversation structures
//! - A runtime engine for processing dialogues during gameplay
//! - Node-based dialogue creation with various node types
//! - Expression evaluation for conditional branches
//! - Variable management across different scopes
//! - UI components for presenting dialogues to players
//! - Integration points with game systems
//! 
//! ## Basic Usage
//! 
//! ```rust
//! use bevy::prelude::*;
//! use funkus_dialogue::DialoguePlugin;
//! 
//! fn main() {
//!     App::new()
//!         .add_plugins((DefaultPlugins, DialoguePlugin))
//!         .run();
//! }
//! ```
//! 
//! For more detailed examples, see the examples directory in the repository.

use bevy::prelude::*;

/// Plugin that sets up the dialogue system components, assets, and systems.
/// 
/// Add this plugin to your Bevy app to enable the dialogue functionality.
pub struct DialoguePlugin;

impl Plugin for DialoguePlugin {
    fn build(&self, app: &mut App) {
        // Plugin initialization will go here
    }
}
