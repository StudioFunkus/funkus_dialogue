//! # Funkus Dialogue UI
//!
//! UI components for displaying dialogues created with the funkus_dialogue system.

mod components;
mod layout;
mod plugin;
mod presentation;
mod systems;

pub use components::*;
pub use layout::{DialogueUIBundle, spawn_dialogue_ui};
pub use plugin::{
    DialogueUIPlugin, INLINE_BADGES_PRESENTATION_KEY, InlineBadgesChoicePresentation,
};
