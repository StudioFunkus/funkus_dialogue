#![deny(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

mod state;

use bevy::prelude::*;
use bevy::prelude::MessageWriter;
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiPrimaryContextPass};

pub use state::{
    apply_editor_commands, DialogueEditorWorkspace, EditorCommand, OpenDialogue,
};

/// Plugin that wires the dialogue editor into an existing Bevy app.
#[derive(Default)]
pub struct DialogueEditorPlugin;

impl Plugin for DialogueEditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DialogueEditorWorkspace>();
        app.add_message::<EditorCommand>();
        app.add_plugins(EguiPlugin::default());
        app.add_systems(Startup, setup_editor_camera);
        app.add_systems(Update, apply_editor_commands);
        app.add_systems(EguiPrimaryContextPass, draw_editor_ui);
    }
}

fn setup_editor_camera(mut commands: Commands) {
    commands.spawn((
        Camera::default(),
        Camera2d,
    ));
}

fn draw_editor_ui(
    mut contexts: EguiContexts,
    workspace: Res<DialogueEditorWorkspace>,
    mut command_writer: MessageWriter<EditorCommand>,
) {
    if let Ok(ctx) = contexts.ctx_mut() {
        egui::TopBottomPanel::top("editor_header").show(ctx, |ui| {
            ui.heading("Funkus Dialogue Editor");
            ui.label(format!(
                "Open dialogues: {}",
                workspace.open_dialogues.len()
            ));
            if ui.button("New Dialogue").clicked() {
                command_writer.write(EditorCommand::NewDialogue {
                    preferred_name: None,
                });
            }
        });

        egui::SidePanel::left("workspace_panel").min_width(220.0).show(ctx, |ui| {
            ui.heading("Open Dialogues");
            if workspace.open_dialogues.is_empty() {
                ui.label("None");
            }

            for (index, dialogue) in workspace.iter_dialogues() {
                let mut label = dialogue.display_name.clone();
                if dialogue.dirty {
                    label.push_str(" *");
                }
                let selected = workspace.active_index == Some(index);
                if ui.selectable_label(selected, label).clicked() {
                    command_writer.write(EditorCommand::SetActive { index });
                }
            }

            ui.separator();
            if let Some(active_idx) = workspace.active_index {
                if ui.button("Close Dialogue").clicked() {
                    command_writer.write(EditorCommand::SetActive { index: active_idx });
                    command_writer.write(EditorCommand::CloseActive { force: true });
                }
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(active) = workspace.active_dialogue() {
                ui.heading(&active.display_name);
                ui.horizontal(|row| {
                    row.label(format!("Nodes: {}", active.graph.node_count()));
                    if let Some(start) = active.graph.start_node {
                        row.label(format!("Start node: {}", start.raw()));
                    } else {
                        row.label("Start node: (not set)");
                    }
                    if let Some(name) = &active.graph.name {
                        row.label(format!("Graph name: {}", name));
                    }
                });
                if active.dirty {
                    ui.colored_label(egui::Color32::YELLOW, "Unsaved changes");
                }
                ui.separator();
                ui.label("Node graph workspace coming soon.");
            } else {
                ui.label("No dialogue file selected yet.");
            }
        });
    }
}
