#![deny(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};

/// Shared session data for the editor.
#[derive(Clone, Debug, Default, Resource)]
pub struct DialogueEditorSession;

/// Plugin that wires the dialogue editor into an existing Bevy app.
#[derive(Default)]
pub struct DialogueEditorPlugin;

impl DialogueEditorPlugin {
    /// Creates a plugin ready to be added to a Bevy app.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Plugin for DialogueEditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DialogueEditorSession>();
        app.add_plugins(EguiPlugin::default());
        app.add_systems(Update, show_placeholder_ui);
    }
}

fn show_placeholder_ui(mut contexts: EguiContexts, _session: Res<DialogueEditorSession>) {
    if let Ok(ctx) = contexts.ctx_mut() {
        egui::TopBottomPanel::top("editor_header").show(ctx, |ui| {
            ui.heading("Funkus Dialogue Editor");
            ui.label("No dialogue file selected yet.");
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Node graph workspace coming soon.");
        });
    }
}
