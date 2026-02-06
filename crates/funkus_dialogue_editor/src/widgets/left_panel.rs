use bevy::prelude::MessageWriter;
use bevy_egui::egui::{self, Ui};
use rfd::FileDialog;

use funkus_dialogue_core::graph::{DialogueElement, NodeId};

use crate::state::{DialogueEditorWorkspace, EditorAssetBrowser, EditorCommand};
use crate::ui_state::{EditorUiState, LeftPanelTab};

pub struct LeftPanelWidget;

pub struct LeftPanelOutput {
    pub selected_node: Option<NodeId>,
}

impl LeftPanelWidget {
    pub fn show(
        &mut self,
        ui: &mut Ui,
        workspace: &DialogueEditorWorkspace,
        asset_browser: &mut EditorAssetBrowser,
        ui_state: &mut EditorUiState,
        command_writer: &mut MessageWriter<EditorCommand>,
    ) -> LeftPanelOutput {
        ui.heading("Open Dialogues");
        if workspace.open_dialogues.is_empty() {
            ui.label("None");
        }

        for (index, dialogue) in workspace.iter_dialogues() {
            let mut label = dialogue.display_name.clone();
            if let Some(path) = dialogue.source_path.as_ref() {
                let file_name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(str::to_owned)
                    .unwrap_or_else(|| path.display().to_string());
                label.push_str(" (");
                label.push_str(&file_name);
                label.push(')');
            } else {
                label.push_str(" (unsaved)");
            }
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

        if let Some(active_idx) = workspace.active_index {
            if ui.button("Close Dialogue").clicked() {
                command_writer.write(EditorCommand::SetActive { index: active_idx });
                command_writer.write(EditorCommand::CloseActive { force: true });
            }
        }

        ui.separator();

        ui.horizontal(|ui| {
            ui.selectable_value(&mut ui_state.left_tab, LeftPanelTab::Assets, "Assets");
            ui.selectable_value(&mut ui_state.left_tab, LeftPanelTab::Nodes, "Nodes");
        });

        ui.separator();

        match ui_state.left_tab {
            LeftPanelTab::Assets => self.show_assets(ui, asset_browser, ui_state, command_writer),
            LeftPanelTab::Nodes => self.show_nodes(ui, workspace, ui_state),
        }
    }

    fn show_assets(
        &mut self,
        ui: &mut Ui,
        asset_browser: &mut EditorAssetBrowser,
        ui_state: &mut EditorUiState,
        command_writer: &mut MessageWriter<EditorCommand>,
    ) -> LeftPanelOutput {
        ui.label(format!(
            "Dialogue directory: {}",
            asset_browser.dialogue_root_display()
        ));
        ui.horizontal(|ui| {
            if ui.button("Refresh").clicked() {
                asset_browser.refresh_assets();
            }
            if ui.button("Load From File...").clicked() {
                let dialog = FileDialog::new()
                    .set_title("Open Dialogue")
                    .set_directory(&asset_browser.dialogue_root)
                    .add_filter("Dialogue Files", &["json", "ron"]);
                if let Some(path) = dialog.pick_file() {
                    command_writer.write(EditorCommand::LoadDialogueFromPath { path });
                }
            }
        });

        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Filter");
            ui.text_edit_singleline(&mut ui_state.asset_filter);
        });

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (index, path) in asset_browser.available_assets.iter().enumerate() {
                let display = path.display().to_string();
                if !ui_state.asset_filter.is_empty()
                    && !display
                        .to_lowercase()
                        .contains(&ui_state.asset_filter.to_lowercase())
                {
                    continue;
                }

                let selected = asset_browser.selected_index == Some(index);
                if ui.selectable_label(selected, display).clicked() {
                    asset_browser.selected_index = Some(index);
                    asset_browser.path_input = path.display().to_string();
                }
            }
        });

        if ui.button("Load Selected").clicked() {
            if let Some(path) = asset_browser.selected_path() {
                command_writer.write(EditorCommand::LoadDialogueFromPath { path });
            }
        }

        LeftPanelOutput {
            selected_node: None,
        }
    }

    fn show_nodes(
        &mut self,
        ui: &mut Ui,
        workspace: &DialogueEditorWorkspace,
        ui_state: &mut EditorUiState,
    ) -> LeftPanelOutput {
        ui.horizontal(|ui| {
            ui.label("Search");
            ui.text_edit_singleline(&mut ui_state.node_filter);
        });

        ui.separator();

        let mut output = LeftPanelOutput {
            selected_node: None,
        };

        if let Some(active) = workspace.active_dialogue() {
            egui::ScrollArea::vertical().show(ui, |ui| {
                let mut ids = active.graph.node_ids();
                ids.sort_by_key(|id| id.raw());

                for id in ids {
                    let Some(node) = active.graph.get_node(id) else {
                        continue;
                    };
                    let label = node.display_name();
                    if !ui_state.node_filter.is_empty()
                        && !label
                            .to_lowercase()
                            .contains(&ui_state.node_filter.to_lowercase())
                    {
                        continue;
                    }

                    let response = ui.selectable_label(false, format!("{}: {}", id.raw(), label));
                    if response.clicked() {
                        output.selected_node = Some(id);
                    }
                }
            });
        } else {
            ui.label("No dialogue loaded.");
        }

        output
    }
}
