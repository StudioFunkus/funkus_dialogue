#![deny(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

mod state;

use bevy::prelude::*;
use bevy::prelude::{MessageReader, MessageWriter};
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass, egui};
use std::path::PathBuf;

pub use state::{
    DialogueEditorWorkspace, EditorAssetBrowser, EditorCommand, OpenDialogue, apply_editor_commands,
};

/// Plugin that wires the dialogue editor into an existing Bevy app.
#[derive(Default)]
pub struct DialogueEditorPlugin;

impl Plugin for DialogueEditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DialogueEditorWorkspace>();
        app.init_resource::<EditorAssetBrowser>();
        app.add_message::<EditorCommand>();
        app.add_plugins(EguiPlugin::default());
        app.add_systems(Startup, setup_editor_camera);
        app.add_systems(
            Update,
            (
                apply_editor_commands,
                handle_editor_io_commands.after(apply_editor_commands),
            ),
        );
        app.add_systems(EguiPrimaryContextPass, draw_editor_ui);
    }
}

fn setup_editor_camera(mut commands: Commands) {
    commands.spawn((Camera::default(), Camera2d));
}

fn draw_editor_ui(
    mut contexts: EguiContexts,
    workspace: Res<DialogueEditorWorkspace>,
    mut asset_browser: ResMut<EditorAssetBrowser>,
    mut command_writer: MessageWriter<EditorCommand>,
) {
    if let Ok(ctx) = contexts.ctx_mut() {
        asset_browser.refresh_stub_assets();

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
            if workspace.has_active() {
                ui.separator();
                if ui.button("Save").clicked() {
                    command_writer.write(EditorCommand::SaveActiveDialogue { destination: None });
                }
                let trimmed = asset_browser.path_input.trim();
                let can_save_as = !trimmed.is_empty();
                if ui
                    .add_enabled(can_save_as, egui::Button::new("Save As"))
                    .clicked()
                {
                    command_writer.write(EditorCommand::SaveActiveDialogue {
                        destination: Some(PathBuf::from(trimmed)),
                    });
                }
            }
        });

        egui::SidePanel::left("workspace_panel")
            .min_width(220.0)
            .show(ctx, |ui| {
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
                        if let Some(path) = dialogue.source_path.as_ref() {
                            asset_browser.select_path(path);
                        } else {
                            asset_browser.selected_index = None;
                            asset_browser.path_input.clear();
                        }
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

        egui::SidePanel::right("asset_browser_panel")
            .min_width(260.0)
            .show(ctx, |ui| {
                ui.heading("Asset Browser");
                ui.label("Enter a path or pick from the list below to load a dialogue.");
                ui.separator();

                ui.label("Manual path:");
                ui.text_edit_singleline(&mut asset_browser.path_input);
                if ui.button("Load Path").clicked() {
                    let trimmed = asset_browser.path_input.trim();
                    if !trimmed.is_empty() {
                        command_writer.write(EditorCommand::LoadDialogueFromPath {
                            path: PathBuf::from(trimmed),
                        });
                    }
                }

                ui.separator();
                ui.label("Known assets:");
                if asset_browser.available_assets.is_empty() {
                    ui.label("No assets registered yet.");
                } else {
                    egui::ScrollArea::vertical().show(ui, |scroll| {
                        let asset_entries: Vec<(usize, String)> = asset_browser
                            .available_assets
                            .iter()
                            .enumerate()
                            .map(|(index, path)| (index, path.display().to_string()))
                            .collect();

                        for (index, display) in asset_entries {
                            let selected = asset_browser.selected_index == Some(index);
                            if scroll.selectable_label(selected, &display).clicked() {
                                asset_browser.selected_index = Some(index);
                                asset_browser.path_input = display;
                            }
                        }
                    });

                    if ui.button("Load Selected").clicked() {
                        if let Some(path) = asset_browser.selected_path() {
                            command_writer.write(EditorCommand::LoadDialogueFromPath { path });
                        }
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
                    if let Some(path) = &active.source_path {
                        row.label(format!("Source: {}", path.display()));
                    } else {
                        row.label("Source: (unsaved)");
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

fn handle_editor_io_commands(
    mut command_reader: MessageReader<EditorCommand>,
    mut workspace: ResMut<DialogueEditorWorkspace>,
    mut asset_browser: ResMut<EditorAssetBrowser>,
) {
    for command in command_reader.read().cloned() {
        match command {
            EditorCommand::LoadDialogueFromPath { path } => {
                let dialogue = OpenDialogue::from_path(path.clone());
                asset_browser.select_path(&path);
                workspace.open_dialogue(dialogue);
            }
            EditorCommand::SaveActiveDialogue { destination } => {
                if let Some(dialogue) = workspace.active_dialogue_mut() {
                    let target_path = destination.or_else(|| dialogue.source_path.clone());

                    if let Some(path) = target_path {
                        dialogue.set_source_path(path.clone());
                        dialogue.dirty = false;
                        asset_browser.select_path(&path);
                    }
                }
            }
            _ => {}
        }
    }
}
