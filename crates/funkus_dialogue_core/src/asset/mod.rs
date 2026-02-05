//! # Dialogue Asset System
//!
//! This module provides the asset definitions and loading functionality for dialogue data.
//!
//! ## Overview
//!
//! The asset system is responsible for:
//!
//! - Defining the `DialogueAsset` type that represents dialogue data
//! - Loading dialogue data from JSON or RON files
//! - Providing access to dialogue data for the runtime system
//!
//! ## Key Components
//!
//! - [`DialogueAsset`]: The main asset type that contains a dialogue graph and metadata
//!
//! ## Usage
//!
//! Dialogue assets are typically loaded through Bevy's asset system. The core plugin
//! registers JSON assets for the `.dialogue.json` extension by default. If you want
//! to load `.dialogue.ron`, register the `bevy_common_assets::ron::RonAssetPlugin`
//! in your app.
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
