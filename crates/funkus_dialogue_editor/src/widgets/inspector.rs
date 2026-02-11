//! Inspector panel for the active dialogue graph and selected node.

use bevy::prelude::{AssetServer, Assets, Image};
use bevy_egui::egui::{self, Ui};
use bevy_egui::{EguiTextureHandle, EguiUserTextures};

use funkus_dialogue_core::DialogueChoicePresentationRegistry;
use funkus_dialogue_core::graph::{DialogueGraph, DialogueNode, NodeId};
use funkus_dialogue_core::registry::{
    DialogueEffect, DialogueFieldKind, DialogueMessageCall, DialogueMessageRegistry,
    DialogueOperation, DialogueRegistry, DialogueValue,
};
use rfd::FileDialog;
use std::collections::BTreeSet;
use std::path::Path;

use crate::node_editor::DialogueNodeEditorState;
use crate::state::{EditorPortraitBrowser, EditorStatusMessages, SUPPORTED_PORTRAIT_EXTENSIONS};

pub struct InspectorWidget;

/// Indicates whether the inspector made changes that should mark the dialogue dirty.
pub struct InspectorOutput {
    pub dirty: bool,
}

enum NodeOutcome {
    Continue,
    Deleted,
}

enum NodeKind {
    Text,
    Choice,
    Effect,
    Message,
}

impl InspectorWidget {
    pub fn show(
        &mut self,
        ui: &mut Ui,
        graph: &mut DialogueGraph,
        node_state: &mut DialogueNodeEditorState,
        status: &mut EditorStatusMessages,
        portrait_browser: &mut EditorPortraitBrowser,
        asset_server: &AssetServer,
        egui_textures: &mut EguiUserTextures,
        images: &Assets<Image>,
        registry: Option<&DialogueRegistry>,
        message_registry: Option<&DialogueMessageRegistry>,
        presentation_registry: Option<&DialogueChoicePresentationRegistry>,
    ) -> InspectorOutput {
        let mut dirty = false;

        dirty |= draw_graph_section(ui, graph);
        ui.separator();

        let Some(node_id) = draw_selection_summary(ui, node_state) else {
            return InspectorOutput { dirty };
        };

        if let NodeOutcome::Deleted = draw_node_actions(ui, graph, node_state, node_id) {
            return InspectorOutput { dirty: true };
        }

        ui.separator();

        let mut node_kind = None;
        if let Some(node) = graph.get_node_mut(node_id) {
            match node {
                DialogueNode::Text {
                    text,
                    speaker,
                    portrait,
                } => {
                    dirty |= draw_text_fields(
                        ui,
                        text,
                        speaker,
                        portrait,
                        portrait_browser,
                        asset_server,
                        egui_textures,
                        images,
                        status,
                    );
                    node_kind = Some(NodeKind::Text);
                }
                DialogueNode::Choice {
                    prompt,
                    presentation_key,
                    speaker,
                    portrait,
                } => {
                    dirty |= draw_choice_fields(
                        ui,
                        prompt,
                        presentation_key,
                        speaker,
                        portrait,
                        portrait_browser,
                        asset_server,
                        egui_textures,
                        images,
                        status,
                        presentation_registry,
                    );
                    node_kind = Some(NodeKind::Choice);
                }
                DialogueNode::Effect { effect } => {
                    dirty |= draw_effect_fields(ui, effect, registry);
                    node_kind = Some(NodeKind::Effect);
                }
                DialogueNode::Message { message } => {
                    dirty |= draw_message_fields(ui, message, message_registry);
                    node_kind = Some(NodeKind::Message);
                }
            }
        }

        if let Some(kind) = node_kind {
            ui.separator();
            match kind {
                NodeKind::Text => {
                    draw_text_connections(ui, graph, node_state, node_id);
                }
                NodeKind::Choice => {
                    dirty |= draw_choice_connections(ui, graph, node_state, status, node_id);
                }
                NodeKind::Effect => {
                    draw_text_connections(ui, graph, node_state, node_id);
                }
                NodeKind::Message => {
                    draw_text_connections(ui, graph, node_state, node_id);
                }
            }
        }

        InspectorOutput { dirty }
    }
}

fn draw_graph_section(ui: &mut Ui, graph: &mut DialogueGraph) -> bool {
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

    dirty
}

fn draw_selection_summary(ui: &mut Ui, node_state: &DialogueNodeEditorState) -> Option<NodeId> {
    match node_state.selected_nodes.as_slice() {
        [] => {
            ui.label("No node selected.");
            None
        }
        [node_id] => {
            ui.heading(format!("Node #{}", node_id.raw()));
            Some(*node_id)
        }
        selection => {
            ui.label(format!("{} nodes selected.", selection.len()));
            None
        }
    }
}

fn draw_node_actions(
    ui: &mut Ui,
    graph: &mut DialogueGraph,
    node_state: &mut DialogueNodeEditorState,
    node_id: NodeId,
) -> NodeOutcome {
    if ui.button("Set As Start Node").clicked() {
        let _ = graph.set_start_node(node_id);
    }

    if ui
        .add(egui::Button::new("Delete Node").fill(egui::Color32::DARK_RED))
        .clicked()
    {
        let _ = graph.remove_node(node_id);
        node_state.drop_selection(node_id);
        return NodeOutcome::Deleted;
    }

    NodeOutcome::Continue
}

fn draw_text_fields(
    ui: &mut Ui,
    text: &mut String,
    speaker: &mut Option<String>,
    portrait: &mut Option<String>,
    portrait_browser: &mut EditorPortraitBrowser,
    asset_server: &AssetServer,
    egui_textures: &mut EguiUserTextures,
    images: &Assets<Image>,
    status: &mut EditorStatusMessages,
) -> bool {
    draw_character_content_fields(
        ui,
        speaker,
        portrait,
        portrait_browser,
        asset_server,
        egui_textures,
        images,
        status,
        "Body",
        |ui| {
            ui.add(egui::TextEdit::multiline(text).desired_rows(4))
                .changed()
        },
    )
}

fn draw_character_content_fields<F>(
    ui: &mut Ui,
    speaker: &mut Option<String>,
    portrait: &mut Option<String>,
    portrait_browser: &mut EditorPortraitBrowser,
    asset_server: &AssetServer,
    egui_textures: &mut EguiUserTextures,
    images: &Assets<Image>,
    status: &mut EditorStatusMessages,
    body_label: &str,
    mut draw_body: F,
) -> bool
where
    F: FnMut(&mut Ui) -> bool,
{
    let mut dirty = false;

    ui.label("Content");
    if edit_optional_field(ui, "Speaker", speaker) {
        dirty = true;
    }

    ui.label(body_label);
    if draw_body(ui) {
        dirty = true;
    }

    dirty |= draw_portrait_picker(
        ui,
        portrait,
        portrait_browser,
        asset_server,
        egui_textures,
        images,
        status,
    );

    dirty
}

fn draw_text_connections(
    ui: &mut Ui,
    graph: &DialogueGraph,
    node_state: &mut DialogueNodeEditorState,
    node_id: NodeId,
) {
    ui.label("Next");
    let connections = graph.get_connected_nodes(node_id);
    if let Some((next, _)) = connections.first() {
        if ui.button(format!("Node #{}", next.raw())).clicked() {
            node_state.request_selection(*next);
        }
    } else {
        ui.label("No outgoing connection.");
    }
}

fn draw_choice_fields(
    ui: &mut Ui,
    prompt: &mut Option<String>,
    presentation_key: &mut Option<String>,
    speaker: &mut Option<String>,
    portrait: &mut Option<String>,
    portrait_browser: &mut EditorPortraitBrowser,
    asset_server: &AssetServer,
    egui_textures: &mut EguiUserTextures,
    images: &Assets<Image>,
    status: &mut EditorStatusMessages,
    presentation_registry: Option<&DialogueChoicePresentationRegistry>,
) -> bool {
    let mut dirty = draw_character_content_fields(
        ui,
        speaker,
        portrait,
        portrait_browser,
        asset_server,
        egui_textures,
        images,
        status,
        "Body",
        |ui| edit_optional_multiline(ui, prompt),
    );

    ui.separator();
    ui.label("Presentation");
    dirty |= draw_choice_presentation_fields(ui, presentation_key, presentation_registry);

    dirty
}

fn draw_choice_presentation_fields(
    ui: &mut Ui,
    presentation_key: &mut Option<String>,
    presentation_registry: Option<&DialogueChoicePresentationRegistry>,
) -> bool {
    let mut dirty = false;

    if let Some(current) = presentation_key.clone() {
        let trimmed = current.trim();
        let normalized = if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        };
        if normalized != *presentation_key {
            *presentation_key = normalized;
            dirty = true;
        }
    }

    ui.label("Presentation Mode");
    if let Some(registry) = presentation_registry {
        let mut known = registry.presentations().collect::<Vec<_>>();
        known.sort_by(|a, b| a.label.cmp(&b.label).then(a.key.cmp(&b.key)));

        let current_key = presentation_key.as_deref();
        let current_is_custom = current_key.is_some_and(|key| !registry.contains(key));
        let selected_text = if let Some(key) = current_key {
            if let Some(definition) = registry.presentation(key) {
                format!("{} ({})", definition.label, definition.key)
            } else {
                format!("Custom ({key})")
            }
        } else {
            "Default (None)".to_string()
        };

        let mut picked_default = false;
        let mut picked_registered_key: Option<String> = None;
        let mut picked_custom = false;

        egui::ComboBox::from_id_salt("choice_presentation_mode")
            .selected_text(selected_text)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(presentation_key.is_none(), "Default (None)")
                    .clicked()
                {
                    picked_default = true;
                }

                if !known.is_empty() {
                    ui.separator();
                    for presentation in &known {
                        let is_selected =
                            presentation_key.as_deref() == Some(presentation.key.as_str());
                        if ui
                            .selectable_label(
                                is_selected,
                                format!("{} ({})", presentation.label, presentation.key),
                            )
                            .clicked()
                        {
                            picked_registered_key = Some(presentation.key.clone());
                        }
                    }
                }

                ui.separator();
                if ui
                    .selectable_label(current_is_custom, "Custom Key")
                    .clicked()
                {
                    picked_custom = true;
                }
            });

        if picked_default && presentation_key.is_some() {
            *presentation_key = None;
            dirty = true;
        }

        if let Some(next_key) = picked_registered_key
            && presentation_key.as_deref() != Some(next_key.as_str())
        {
            *presentation_key = Some(next_key);
            dirty = true;
        }

        if picked_custom && !current_is_custom {
            *presentation_key = Some("custom_mode".to_string());
            dirty = true;
        }

        if let Some(key) = presentation_key.as_deref() {
            if let Some(definition) = registry.presentation(key) {
                if let Some(description) = definition.description.as_deref() {
                    ui.small(description);
                } else {
                    ui.small("Registered mode");
                }
            } else {
                if key == "custom_mode" {
                    ui.small("Replace \"custom_mode\" with your game-specific key.");
                }
                let mut custom_key = key.to_string();
                ui.horizontal(|ui| {
                    ui.label("Custom Key");
                    if ui.text_edit_singleline(&mut custom_key).changed() {
                        let trimmed = custom_key.trim();
                        *presentation_key = if trimmed.is_empty() {
                            None
                        } else {
                            Some(trimmed.to_string())
                        };
                        dirty = true;
                    }
                });
            }
        } else {
            ui.small("No presentation key set.");
        }
    } else {
        ui.small("Leave empty to use default behavior.");
        if edit_optional_field(ui, "Presentation Key (optional)", presentation_key) {
            dirty = true;
        }
    }

    dirty
}

fn draw_choice_connections(
    ui: &mut Ui,
    graph: &mut DialogueGraph,
    node_state: &mut DialogueNodeEditorState,
    status: &mut EditorStatusMessages,
    node_id: NodeId,
) -> bool {
    let mut dirty = false;

    ui.label("Choices");
    ui.small("Optional choice_key values provide stable IDs for gameplay logic.");
    let connections = graph.get_outgoing_connections(node_id);
    if connections.is_empty() {
        ui.label("No outgoing connections.");
        return dirty;
    }

    let mut reorder_request: Option<(usize, usize)> = None;
    for (index, (target, connection_data)) in connections.iter().enumerate() {
        let mut current = connection_data
            .label
            .clone()
            .unwrap_or_else(|| format!("Choice {}", index + 1));
        let mut choice_key = connection_data.choice_key.clone().unwrap_or_default();
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label("Move");
                let is_first = index == 0;
                let is_last = index + 1 == connections.len();
                if ui.add_enabled(!is_first, egui::Button::new("^")).clicked() {
                    reorder_request = Some((index, index.saturating_sub(1)));
                }
                if ui.add_enabled(!is_last, egui::Button::new("v")).clicked() {
                    reorder_request = Some((index, index + 1));
                }
                if ui.button(format!("Node #{}", target.raw())).clicked() {
                    node_state.request_selection(*target);
                }
            });

            ui.horizontal(|ui| {
                ui.label("Choice Text");
                if ui.text_edit_singleline(&mut current).changed() {
                    let trimmed = current.trim();
                    let updated = if trimmed.is_empty() {
                        None
                    } else {
                        Some(current.clone())
                    };
                    let _ = graph.update_connection_label(node_id, *target, updated);
                    dirty = true;
                }
            });

            let is_last = index + 1 == connections.len();
            ui.horizontal(|ui| {
                ui.label("Choice Key");
                if ui.text_edit_singleline(&mut choice_key).changed() {
                    let trimmed = choice_key.trim();
                    let updated = if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    };
                    let _ = graph.update_connection_choice_key(node_id, *target, updated);
                    dirty = true;
                }
            });

            if !is_last {
                ui.add_space(2.0);
            }
        });
    }

    let mut seen = BTreeSet::new();
    let mut duplicates = BTreeSet::new();
    for (_, data) in &connections {
        let Some(choice_key) = data.choice_key.as_deref() else {
            continue;
        };
        if !seen.insert(choice_key.to_string()) {
            duplicates.insert(choice_key.to_string());
        }
    }
    if !duplicates.is_empty() {
        let joined = duplicates.into_iter().collect::<Vec<_>>().join(", ");
        ui.colored_label(
            egui::Color32::YELLOW,
            format!("Duplicate choice_key values: {joined}"),
        );
    }

    if let Some((from, to)) = reorder_request {
        let mut ordered_targets: Vec<_> = connections.iter().map(|(target, _)| *target).collect();
        ordered_targets.swap(from, to);
        match graph.reorder_connections(node_id, &ordered_targets) {
            Ok(()) => {
                node_state.refresh_connections_for_node(graph, node_id);
                dirty = true;
            }
            Err(error) => {
                status.error(format!(
                    "Failed to reorder choices for node #{}: {error}",
                    node_id.raw()
                ));
            }
        }
    }

    dirty
}

fn draw_effect_fields(
    ui: &mut Ui,
    effect: &mut DialogueEffect,
    registry: Option<&DialogueRegistry>,
) -> bool {
    let mut dirty = false;

    ui.heading("Effect");

    if let Some(registry) = registry {
        let mut keys: Vec<&String> = registry.fields().map(|field| &field.key).collect();
        keys.sort();
        let current_key = effect.key.clone();
        egui::ComboBox::from_id_salt("effect_key")
            .selected_text(effect.key.as_str())
            .show_ui(ui, |ui| {
                for key in keys {
                    if ui.selectable_label(effect.key == *key, key).clicked() {
                        effect.key = key.clone();
                        dirty = true;
                    }
                }
            });
        if effect.key.is_empty() {
            effect.key = current_key;
        }
    } else {
        dirty |= ui.text_edit_singleline(&mut effect.key).changed();
    }

    let field_kind =
        registry.and_then(|registry| registry.field(&effect.key).map(|f| f.kind.clone()));

    if let Some(kind) = field_kind.as_ref() {
        dirty |= ensure_value_kind(effect, kind, effect.op);
    }

    let allowed_ops = allowed_operations(field_kind.as_ref());
    if !allowed_ops.contains(&effect.op) {
        effect.op = allowed_ops[0];
        dirty = true;
    }

    ui.horizontal(|ui| {
        ui.label("Operation");
        egui::ComboBox::from_id_salt("effect_op")
            .selected_text(format!("{:?}", effect.op))
            .show_ui(ui, |ui| {
                for op in allowed_ops {
                    if ui
                        .selectable_label(effect.op == op, format!("{:?}", op))
                        .clicked()
                    {
                        effect.op = op;
                        dirty = true;
                    }
                }
            });
    });

    ui.separator();
    ui.label("Value");

    match field_kind.as_ref() {
        Some(DialogueFieldKind::Bool) => {
            let mut value = match effect.value {
                DialogueValue::Bool(v) => v,
                _ => false,
            };
            if ui.checkbox(&mut value, "Enabled").changed() {
                effect.value = DialogueValue::Bool(value);
                dirty = true;
            }
        }
        Some(DialogueFieldKind::Int) => {
            let mut value = match effect.value {
                DialogueValue::Int(v) => v,
                _ => 0,
            };
            if ui.add(egui::DragValue::new(&mut value)).changed() {
                effect.value = DialogueValue::Int(value);
                dirty = true;
            }
        }
        Some(DialogueFieldKind::Float) => {
            let mut value = match effect.value {
                DialogueValue::Float(v) => v,
                _ => 0.0,
            };
            if ui
                .add(egui::DragValue::new(&mut value).speed(0.1))
                .changed()
            {
                effect.value = DialogueValue::Float(value);
                dirty = true;
            }
        }
        Some(DialogueFieldKind::String) => {
            let mut value = match effect.value.clone() {
                DialogueValue::String(v) => v,
                _ => String::new(),
            };
            if ui.text_edit_singleline(&mut value).changed() {
                effect.value = DialogueValue::String(value);
                dirty = true;
            }
        }
        Some(DialogueFieldKind::Enum { variants }) => {
            let mut value = match effect.value.clone() {
                DialogueValue::Enum(v) => v,
                _ => variants.first().cloned().unwrap_or_default(),
            };
            egui::ComboBox::from_id_salt("effect_enum_value")
                .selected_text(value.clone())
                .show_ui(ui, |ui| {
                    for variant in variants {
                        if ui.selectable_label(&value == variant, variant).clicked() {
                            value = variant.clone();
                        }
                    }
                });
            if value
                != match &effect.value {
                    DialogueValue::Enum(v) => v,
                    _ => "",
                }
            {
                effect.value = DialogueValue::Enum(value);
                dirty = true;
            }
        }
        Some(DialogueFieldKind::List(inner)) => {
            dirty |= draw_list_value(ui, effect, inner, effect.op);
        }
        Some(DialogueFieldKind::Array { element, len }) => {
            ui.label(format!("Array length: {len}"));
            dirty |= draw_list_value(ui, effect, element, DialogueOperation::Set);
        }
        None => {
            ui.label("No registry entry for key. Storing value as string.");
            let mut value = match effect.value.clone() {
                DialogueValue::String(v) => v,
                _ => String::new(),
            };
            if ui.text_edit_singleline(&mut value).changed() {
                effect.value = DialogueValue::String(value);
                dirty = true;
            }
        }
    }

    dirty
}

fn draw_message_fields(
    ui: &mut Ui,
    message: &mut DialogueMessageCall,
    registry: Option<&DialogueMessageRegistry>,
) -> bool {
    let mut dirty = false;

    ui.heading("Message");

    if let Some(registry) = registry {
        let mut keys: Vec<&String> = registry
            .messages()
            .map(|definition| &definition.key)
            .collect();
        keys.sort();

        let previous = message.key.clone();
        egui::ComboBox::from_id_salt("message_key")
            .selected_text(message.key.as_str())
            .show_ui(ui, |ui| {
                for key in keys {
                    if ui.selectable_label(message.key == *key, key).clicked() {
                        message.key = key.clone();
                    }
                }
            });

        if message.key.is_empty() {
            message.key = previous;
        } else if message.key != previous {
            dirty = true;
        }

        let Some(definition) = registry.message(&message.key) else {
            ui.label("No message metadata for this key.");
            return dirty;
        };

        for field in &definition.fields {
            let index = if let Some(index) = message
                .params
                .iter()
                .position(|param| param.name == field.name)
            {
                index
            } else {
                message
                    .params
                    .push(funkus_dialogue_core::registry::DialogueMessageParam {
                        name: field.name.clone(),
                        value: default_value_for_kind(&field.kind),
                    });
                dirty = true;
                message.params.len().saturating_sub(1)
            };

            if !value_matches_kind(&message.params[index].value, &field.kind) {
                message.params[index].value = default_value_for_kind(&field.kind);
                dirty = true;
            }

            ui.separator();
            ui.label(format!("{} ({:?})", field.name, field.kind));
            if edit_message_field_value(ui, &mut message.params[index].value, &field.kind) {
                dirty = true;
            }
        }
    } else {
        ui.label("No dialogue message registry available.");
        dirty |= ui.text_edit_singleline(&mut message.key).changed();
    }

    dirty
}

fn edit_message_field_value(
    ui: &mut Ui,
    value: &mut DialogueValue,
    kind: &DialogueFieldKind,
) -> bool {
    let mut dirty = false;

    if !value_matches_kind(value, kind) {
        *value = default_value_for_kind(kind);
        dirty = true;
    }

    match kind {
        DialogueFieldKind::Bool
        | DialogueFieldKind::Int
        | DialogueFieldKind::Float
        | DialogueFieldKind::String
        | DialogueFieldKind::Enum { .. } => {
            dirty |= edit_value(ui, value, kind);
        }
        DialogueFieldKind::List(inner) => {
            let DialogueValue::List(items) = value else {
                return true;
            };

            let mut remove_index = None;
            for (index, item) in items.iter_mut().enumerate() {
                ui.push_id(index, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(format!("#{}", index + 1));
                        if edit_message_field_value(ui, item, inner) {
                            dirty = true;
                        }
                        if ui.button("Remove").clicked() {
                            remove_index = Some(index);
                        }
                    });
                });
            }

            if let Some(index) = remove_index {
                items.remove(index);
                dirty = true;
            }

            if ui.button("Add Item").clicked() {
                items.push(default_value_for_kind(inner));
                dirty = true;
            }
        }
        DialogueFieldKind::Array { element, len } => {
            let DialogueValue::List(items) = value else {
                return true;
            };

            if items.len() != *len {
                *items = (0..*len).map(|_| default_value_for_kind(element)).collect();
                dirty = true;
            }

            for (index, item) in items.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!("#{}", index + 1));
                    if edit_message_field_value(ui, item, element) {
                        dirty = true;
                    }
                });
            }
        }
    }

    dirty
}

fn draw_list_value(
    ui: &mut Ui,
    effect: &mut DialogueEffect,
    inner: &DialogueFieldKind,
    op: DialogueOperation,
) -> bool {
    let mut dirty = false;
    if matches!(op, DialogueOperation::Clear) {
        ui.label("Value not required for clear.");
        return dirty;
    }

    if matches!(op, DialogueOperation::Push | DialogueOperation::Remove) {
        let value_ref = match &mut effect.value {
            DialogueValue::Bool(_)
            | DialogueValue::Int(_)
            | DialogueValue::Float(_)
            | DialogueValue::String(_)
            | DialogueValue::Enum(_) => &mut effect.value,
            _ => {
                effect.value = default_value_for_kind(inner);
                &mut effect.value
            }
        };
        dirty |= edit_value(ui, value_ref, inner);
        return dirty;
    }

    let DialogueValue::List(ref mut items) = effect.value else {
        effect.value = DialogueValue::List(Vec::new());
        return true;
    };

    let mut remove_index = None;

    for (index, item) in items.iter_mut().enumerate() {
        ui.push_id(index, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("#{}", index + 1));
                if edit_value(ui, item, inner) {
                    dirty = true;
                }
                if ui.button("Remove").clicked() {
                    remove_index = Some(index);
                }
            });
        });
    }

    if let Some(index) = remove_index {
        items.remove(index);
        dirty = true;
    }

    if ui.button("Add Item").clicked() {
        items.push(default_value_for_kind(inner));
        dirty = true;
    }

    dirty
}

fn edit_value(ui: &mut Ui, value: &mut DialogueValue, kind: &DialogueFieldKind) -> bool {
    match (value, kind) {
        (DialogueValue::Bool(v), DialogueFieldKind::Bool) => ui.checkbox(v, "").changed(),
        (DialogueValue::Int(v), DialogueFieldKind::Int) => {
            ui.add(egui::DragValue::new(v)).changed()
        }
        (DialogueValue::Float(v), DialogueFieldKind::Float) => {
            ui.add(egui::DragValue::new(v).speed(0.1)).changed()
        }
        (DialogueValue::String(v), DialogueFieldKind::String) => {
            ui.text_edit_singleline(v).changed()
        }
        (DialogueValue::Enum(v), DialogueFieldKind::Enum { variants }) => {
            let mut changed = false;
            egui::ComboBox::from_id_salt("effect_list_enum_value")
                .selected_text(v.clone())
                .show_ui(ui, |ui| {
                    for variant in variants {
                        if ui.selectable_label(&*v == variant, variant).clicked() {
                            *v = variant.clone();
                            changed = true;
                        }
                    }
                });
            changed
        }
        _ => false,
    }
}

fn value_matches_kind(value: &DialogueValue, kind: &DialogueFieldKind) -> bool {
    match (value, kind) {
        (DialogueValue::Bool(_), DialogueFieldKind::Bool)
        | (DialogueValue::Int(_), DialogueFieldKind::Int)
        | (DialogueValue::Float(_), DialogueFieldKind::Float)
        | (DialogueValue::String(_), DialogueFieldKind::String)
        | (DialogueValue::Enum(_), DialogueFieldKind::Enum { .. })
        | (DialogueValue::List(_), DialogueFieldKind::List(_))
        | (DialogueValue::List(_), DialogueFieldKind::Array { .. }) => true,
        _ => false,
    }
}

fn ensure_value_kind(
    effect: &mut DialogueEffect,
    kind: &DialogueFieldKind,
    op: DialogueOperation,
) -> bool {
    if !value_matches_kind(&effect.value, kind) {
        effect.value = default_value_for_kind(kind);
        return true;
    }

    if let DialogueFieldKind::List(inner) = kind {
        if matches!(op, DialogueOperation::Push | DialogueOperation::Remove)
            && matches!(effect.value, DialogueValue::List(_))
        {
            effect.value = default_value_for_kind(inner);
            return true;
        }
    }
    false
}

fn default_value_for_kind(kind: &DialogueFieldKind) -> DialogueValue {
    match kind {
        DialogueFieldKind::Bool => DialogueValue::Bool(false),
        DialogueFieldKind::Int => DialogueValue::Int(0),
        DialogueFieldKind::Float => DialogueValue::Float(0.0),
        DialogueFieldKind::String => DialogueValue::String(String::new()),
        DialogueFieldKind::Enum { variants } => {
            DialogueValue::Enum(variants.first().cloned().unwrap_or_default())
        }
        DialogueFieldKind::List(inner) => DialogueValue::List(vec![default_value_for_kind(inner)]),
        DialogueFieldKind::Array { element, len } => {
            DialogueValue::List((0..*len).map(|_| default_value_for_kind(element)).collect())
        }
    }
}

fn allowed_operations(kind: Option<&DialogueFieldKind>) -> Vec<DialogueOperation> {
    match kind {
        Some(kind) => kind.allowed_operations().to_vec(),
        None => vec![
            DialogueOperation::Set,
            DialogueOperation::Add,
            DialogueOperation::Subtract,
            DialogueOperation::Toggle,
            DialogueOperation::Push,
            DialogueOperation::Remove,
            DialogueOperation::Clear,
        ],
    }
}

fn draw_portrait_picker(
    ui: &mut Ui,
    portrait: &mut Option<String>,
    portrait_browser: &mut EditorPortraitBrowser,
    asset_server: &AssetServer,
    egui_textures: &mut EguiUserTextures,
    images: &Assets<Image>,
    status: &mut EditorStatusMessages,
) -> bool {
    let mut dirty = false;

    ui.label("Portrait");
    ui.horizontal(|ui| {
        let selected_label = portrait.as_deref().unwrap_or("None");
        egui::ComboBox::from_id_salt("portrait_selector")
            .selected_text(selected_label)
            .show_ui(ui, |ui| {
                if ui.selectable_label(portrait.is_none(), "None").clicked() {
                    *portrait = None;
                    dirty = true;
                }

                for path in &portrait_browser.available_assets {
                    let label = asset_path_string(path);
                    let is_selected = portrait.as_deref() == Some(label.as_str());
                    if ui.selectable_label(is_selected, &label).clicked() {
                        *portrait = Some(label);
                        dirty = true;
                    }
                }
            });

        if ui.button("Import").clicked() {
            if let Some(path) = FileDialog::new()
                .add_filter("Images", SUPPORTED_PORTRAIT_EXTENSIONS)
                .pick_file()
            {
                match portrait_browser.import_into_portrait_root(&path) {
                    Ok(imported) => {
                        let relative = portrait_browser.relative_path_if_within_assets(&imported);
                        *portrait = Some(asset_path_string(&relative));
                        status.success(format!("Imported portrait {}", relative.display()));
                        dirty = true;
                    }
                    Err(error) => {
                        status.error(format!(
                            "Failed to import portrait {}: {error}",
                            path.display()
                        ));
                    }
                }
            }
        }

        if ui.button("Clear").clicked() {
            *portrait = None;
            dirty = true;
        }
    });

    if let Some(path) = portrait.as_ref() {
        let handle = portrait_browser.load_handle(asset_server, path);
        if images.get(&handle).is_some() {
            let texture_id = egui_textures.add_image(EguiTextureHandle::Weak(handle.id()));
            let size = egui::vec2(64.0, 64.0);
            ui.image((texture_id, size));
        } else {
            ui.label("Loading portrait...");
        }
    }

    dirty
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

fn asset_path_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
