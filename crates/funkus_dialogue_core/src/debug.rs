//! Debug utilities for the dialogue system.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};

use crate::{
    graph::NodeId,
    runtime::{DialogueRunner, DialogueState},
};

/// Plugin for dialogue system debugging tools.
pub struct DialogueDebugPlugin;

impl Plugin for DialogueDebugPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<EguiPlugin>() {
            app.add_plugins(EguiPlugin {
                enable_multipass_for_primary_context: true,
            });
        }
        app.register_type::<DialogueState>()
            .register_type::<NodeId>()
            .register_type::<Option<NodeId>>()
            .init_resource::<DialogueDebugState>()
            .add_systems(Update, debug_ui_system);

        info!("Dialogue Debug UI enabled - press F1 to toggle");
    }
}

/// Dialogue debug UI state
#[derive(Default, Resource)]
pub struct DialogueDebugState {
    /// Whether the debug UI is visible
    pub visible: bool,
    /// ID of the currently selected entity
    pub selected_entity: Option<Entity>,
}

/// System that displays debug information about dialogue.
fn debug_ui_system(
    mut state: ResMut<DialogueDebugState>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    dialogue_runners: Query<(Entity, &DialogueRunner, &Name)>,
    mut contexts: EguiContexts,
) {
    // Toggle debug UI with F1
    if keyboard_input.just_pressed(KeyCode::F1) {
        state.visible = !state.visible;
        info!(
            "Dialogue Debug UI {}",
            if state.visible { "shown" } else { "hidden" }
        );
    }

    // Skip if UI is hidden
    if !state.visible {
        return;
    }

    // Create a very simple debug window
    egui::Window::new("Dialogue Debug").show(contexts.ctx_mut(), |ui| {
        ui.heading("Dialogue Entities");

        // List all dialogue entities
        for (entity, runner, name) in dialogue_runners.iter() {
            let text = format!("{} ({:?}) - State: {:?}", name, entity, runner.state);
            if ui.button(text).clicked() {
                state.selected_entity = Some(entity);
            }
        }

        if dialogue_runners.is_empty() {
            ui.label("No dialogue entities found");
        }
    });
}
