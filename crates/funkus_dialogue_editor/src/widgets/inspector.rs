use bevy_egui::egui::{self, Ui};

use funkus_dialogue_core::graph::{DialogueGraph, DialogueNode};

use crate::node_editor::DialogueNodeEditorState;
use crate::state::EditorStatusMessages;

pub struct InspectorWidget;

pub struct InspectorOutput {
    pub dirty: bool,
}

impl InspectorWidget {
    pub fn show(
        &mut self,
        ui: &mut Ui,
        graph: &mut DialogueGraph,
        node_state: &mut DialogueNodeEditorState,
        status: &mut EditorStatusMessages,
    ) -> InspectorOutput {
        let mut dirty = false;

        ui.heading("Graph");
        ui.horizontal(|ui| {
            ui.label("Name");
            let mut name = graph.name.clone().unwrap_or_default();
            if ui.text_edit_singleline(&mut name).changed() {
                graph.name = if name.trim().is_empty() {
                    None
                } else {
                    Some(name)
                };
                dirty = true;
            }
        });

        ui.horizontal(|ui| {
            ui.label("Start node");
            if let Some(start) = graph.start_node {
                ui.label(format!("#{}", start.raw()));
                if ui.small_button("Clear").clicked() {
                    graph.clear_start_node();
                    dirty = true;
                }
            } else {
                ui.label("Not set");
            }
        });

        if ui.button("Validate Graph").clicked() {
            match graph.validate() {
                Ok(()) => status.success("Graph validated successfully."),
                Err(error) => status.error(format!("Validation failed: {error}")),
            }
        }

        ui.separator();

        if node_state.selected_nodes.is_empty() {
            ui.label("No node selected.");
            return InspectorOutput { dirty };
        }

        if node_state.selected_nodes.len() > 1 {
            ui.label(format!(
                "{} nodes selected.",
                node_state.selected_nodes.len()
            ));
            return InspectorOutput { dirty };
        }

        let node_id = node_state.selected_nodes[0];
        ui.heading(format!("Node #{}", node_id.raw()));

        if ui.button("Set As Start Node").clicked() {
            let _ = graph.set_start_node(node_id);
            dirty = true;
        }

        if ui
            .add(egui::Button::new("Delete Node").fill(egui::Color32::DARK_RED))
            .clicked()
        {
            let _ = graph.remove_node(node_id);
            node_state.drop_selection(node_id);
            dirty = true;
            return InspectorOutput { dirty };
        }

        ui.separator();

        if let Some(node) = graph.get_node_mut(node_id) {
            match node {
                DialogueNode::Text {
                    text,
                    speaker,
                    portrait,
                } => {
                    ui.label("Dialogue Text");
                    if ui
                        .add(egui::TextEdit::multiline(text).desired_rows(4))
                        .changed()
                    {
                        dirty = true;
                    }

                    if edit_optional_field(ui, "Speaker", speaker) {
                        dirty = true;
                    }
                    if edit_optional_field(ui, "Portrait", portrait) {
                        dirty = true;
                    }

                    ui.separator();
                    ui.label("Next");
                    let connections = graph.get_connected_nodes(node_id);
                    if let Some((next, _)) = connections.first() {
                        ui.label(format!("-> #{}", next.raw()));
                    } else {
                        ui.label("No outgoing connection.");
                    }
                }
                DialogueNode::Choice {
                    prompt,
                    speaker,
                    portrait,
                } => {
                    ui.label("Prompt");
                    if edit_optional_multiline(ui, prompt) {
                        dirty = true;
                    }

                    if edit_optional_field(ui, "Speaker", speaker) {
                        dirty = true;
                    }
                    if edit_optional_field(ui, "Portrait", portrait) {
                        dirty = true;
                    }

                    ui.separator();
                    ui.label("Choices");

                    let mut connections = graph.get_connected_nodes(node_id);
                    if connections.is_empty() {
                        ui.label("No outgoing connections.");
                    } else {
                        connections.sort_by(|a, b| a.0.raw().cmp(&b.0.raw()));
                        for (index, (target, label)) in connections.iter().enumerate() {
                            let mut current = label
                                .clone()
                                .unwrap_or_else(|| format!("Choice {}", index + 1));
                            ui.horizontal(|ui| {
                                ui.label(format!("→ #{}", target.raw()));
                                if ui.text_edit_singleline(&mut current).changed() {
                                    let trimmed = current.trim();
                                    let updated = if trimmed.is_empty() {
                                        None
                                    } else {
                                        Some(current.clone())
                                    };
                                    let _ =
                                        graph.update_connection_label(node_id, *target, updated);
                                    dirty = true;
                                }
                            });
                        }
                    }
                }
            }
        }

        InspectorOutput { dirty }
    }
}

fn edit_optional_field(ui: &mut Ui, label: &str, value: &mut Option<String>) -> bool {
    let mut buffer = value.clone().unwrap_or_default();
    let response = ui.horizontal(|ui| {
        ui.label(label);
        ui.text_edit_singleline(&mut buffer)
    });

    if response.inner.changed() {
        let trimmed = buffer.trim();
        if trimmed.is_empty() {
            *value = None;
        } else {
            *value = Some(buffer);
        }
        return true;
    }

    false
}

fn edit_optional_multiline(ui: &mut Ui, value: &mut Option<String>) -> bool {
    let mut buffer = value.clone().unwrap_or_default();
    let response = ui.add(egui::TextEdit::multiline(&mut buffer).desired_rows(3));
    if response.changed() {
        let trimmed = buffer.trim();
        if trimmed.is_empty() {
            *value = None;
        } else {
            *value = Some(buffer);
        }
        return true;
    }
    false
}
