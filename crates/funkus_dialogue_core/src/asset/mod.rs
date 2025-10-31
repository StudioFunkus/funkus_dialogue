//! # Dialogue Asset System
//!
//! This module provides the asset definitions and loading functionality for dialogue data.
//!
//! ## Overview
//!
//! The asset system is responsible for:
//!
//! - Defining the `DialogueAsset` type that represents dialogue data
//! - Loading dialogue data from JSON files
//! - Providing access to dialogue data for the runtime system
//!
//! ## Key Components
//!
//! - [`DialogueAsset`]: The main asset type that contains a dialogue graph and metadata
//!
//! ## Usage
//!
//! Dialogue assets are typically loaded through Bevy's asset system:
//!
//! ```rust,ignore
//! use bevy::prelude::*;
//! use funkus_dialogue_core::asset::DialogueAsset;
//!
//! fn setup(asset_server: Res<AssetServer>) {
//!     let dialogue_handle: Handle<DialogueAsset> =
//!         asset_server.load("dialogues/example.dialogue.json");
//!     let _unused = dialogue_handle;
//! }
//! ```

mod dialogue_asset;

pub use dialogue_asset::*;
