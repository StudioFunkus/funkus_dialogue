use bevy::prelude::MessageWriter;
use bevy_egui::egui::{self, Ui};
use rfd::FileDialog;
use std::path::PathBuf;

use crate::state::{
    DialogueEditorWorkspace, EditorAssetBrowser, EditorCommand, EditorStatusMessages,
};

pub struct ToolbarWidget;

impl ToolbarWidget {
    pub fn show(
        &mut self,
        ui: &mut Ui,
        workspace: &DialogueEditorWorkspace,
        asset_browser: &EditorAssetBrowser,
        status: &mut EditorStatusMessages,
        command_writer: &mut MessageWriter<EditorCommand>,
    ) {
        ui.horizontal(|ui| {
            if ui.button("New").clicked() {
                command_writer.write(EditorCommand::NewDialogue {
                    preferred_name: None,
                });
            }

            if ui.button("Open...").clicked() {
                let dialog = FileDialog::new()
                    .set_title("Open Dialogue")
                    .set_directory(&asset_browser.dialogue_root)
                    .add_filter("Dialogue Files", &["json", "ron"]);
                if let Some(path) = dialog.pick_file() {
                    command_writer.write(EditorCommand::LoadDialogueFromPath { path });
                }
            }

            let can_save_existing = workspace
                .active_dialogue()
                .and_then(|dialogue| dialogue.source_path.as_ref())
                .is_some();

            if ui
                .add_enabled(can_save_existing, egui::Button::new("Save"))
                .clicked()
            {
                command_writer.write(EditorCommand::SaveActiveDialogue { destination: None });
            }

            if ui.button("Save As...").clicked() {
                let mut dialog = FileDialog::new()
                    .set_title("Save Dialogue")
                    .set_directory(&asset_browser.dialogue_root)
                    .add_filter("Dialogue Files", &["json", "ron"]);

                if let Some(active) = workspace.active_dialogue() {
                    if let Some(existing) = active.source_path.as_ref() {
                        let absolute = asset_browser.to_absolute_dialogue_path(existing);
                        if let Some(parent) = absolute.parent() {
                            dialog = dialog.set_directory(parent);
                        }
                        if let Some(file_name) = absolute.file_name().and_then(|name| name.to_str())
                        {
                            dialog = dialog.set_file_name(file_name);
                        }
                    } else {
                        let default_name =
                            format!("{}.json", active.display_name.replace([' ', '\t'], "_"));
                        dialog = dialog.set_file_name(&default_name);
                    }
                }

                if let Some(path) = dialog.save_file() {
                    command_writer.write(EditorCommand::SaveActiveDialogue {
                        destination: Some(ensure_dialogue_extension(path)),
                    });
                }
            }

            ui.separator();

            let can_validate = workspace.active_dialogue().is_some();
            if ui
                .add_enabled(can_validate, egui::Button::new("Validate"))
                .clicked()
            {
                if let Some(active) = workspace.active_dialogue() {
                    match active.graph.validate() {
                        Ok(()) => status.success("Dialogue graph validated successfully."),
                        Err(error) => status.error(format!("Graph validation failed: {error}")),
                    }
                }
            }
        });
    }
}

fn ensure_dialogue_extension(mut path: PathBuf) -> PathBuf {
    if path.extension().is_none() {
        path.set_extension("json");
    }
    path
}
