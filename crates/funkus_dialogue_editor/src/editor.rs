use bevy::prelude::*;
use bevy_egui::{egui, EguiContextPass, EguiContexts};
use egui_snarl::{ui::SnarlStyle, NodeId as SnarlNodeId, Snarl};
use funkus_dialogue_core::{
    graph::{ConnectionData, DialogueGraph, NodeId},
    DialogueAsset, DialogueNode,
};
use std::collections::HashMap;

use crate::node_wrapper::EditorNode;
use crate::viewer::DialogueViewer;

/// Resource to store editor state
#[derive(Resource)]
pub struct DialogueEditorState {
    /// The Snarl editor state for dialogue nodes
    pub snarl: Snarl<EditorNode>,
    /// Current dialogue name being edited
    pub current_dialogue_name: String,
    /// Whether the editor window is visible
    pub visible: bool,
    /// Mapping between Snarl NodeIds and our NodeIds
    pub id_mapping: HashMap<SnarlNodeId, NodeId>,
    /// Reverse mapping from our NodeIds to Snarl NodeIds
    pub reverse_id_mapping: HashMap<NodeId, SnarlNodeId>,
    /// Next node ID to assign
    pub next_node_id: u32,
    /// Track all snarl node IDs
    pub snarl_node_ids: Vec<SnarlNodeId>,
}

impl Default for DialogueEditorState {
    fn default() -> Self {
        Self {
            snarl: Snarl::new(),
            current_dialogue_name: "New Dialogue".to_string(),
            visible: true,
            id_mapping: HashMap::new(),
            reverse_id_mapping: HashMap::new(),
            next_node_id: 1,
            snarl_node_ids: Vec::new(),
        }
    }
}

/// Plugin for the dialogue editor functionality
pub struct DialogueEditorPlugin;

impl Plugin for DialogueEditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DialogueEditorState>()
            .add_systems(EguiContextPass, dialogue_editor_system);
    }
}

/// Main system for the dialogue editor UI
fn dialogue_editor_system(
    mut contexts: EguiContexts,
    mut editor_state: ResMut<DialogueEditorState>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    asset_server: Res<AssetServer>,
) {
    // Toggle editor with F2
    if keyboard_input.just_pressed(KeyCode::F2) {
        editor_state.visible = !editor_state.visible;
    }

    if !editor_state.visible {
        return;
    }

    let ctx = contexts.ctx_mut();

    egui::Window::new("Dialogue Editor")
        .default_size([800.0, 600.0])
        .show(ctx, |ui| {
            // Top toolbar
            ui.horizontal(|ui| {
                ui.label("Dialogue Name:");
                ui.text_edit_singleline(&mut editor_state.current_dialogue_name);

                ui.separator();

                if ui.button("Save").clicked() {
                    save_dialogue(&editor_state, &asset_server);
                }

                if ui.button("Load").clicked() {
                    // TODO: Implement load functionality
                    info!("Load functionality not yet implemented");
                }

                if ui.button("Clear").clicked() {
                    editor_state.snarl = Snarl::new();
                    editor_state.id_mapping.clear();
                    editor_state.reverse_id_mapping.clear();
                    editor_state.snarl_node_ids.clear();
                    editor_state.next_node_id = 1;
                }
            });

            ui.separator();

            // Add initial node if empty
            if editor_state.snarl_node_ids.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.label("Empty dialogue. Right-click to add nodes.");

                    if ui.button("Add Start Node").clicked() {
                        let node_id = NodeId(editor_state.next_node_id);
                        editor_state.next_node_id += 1;

                        let node = DialogueNode::text(node_id, "Start of dialogue")
                            .with_speaker("Narrator");
                        let editor_node = EditorNode::new(node, (400.0, 300.0));

                        let snarl_id = editor_state
                            .snarl
                            .insert_node(egui::pos2(400.0, 300.0), editor_node);

                        editor_state.id_mapping.insert(snarl_id, node_id);
                        editor_state.reverse_id_mapping.insert(node_id, snarl_id);
                        editor_state.snarl_node_ids.push(snarl_id);
                    }
                });
            }

            // The node editor
            let style = SnarlStyle::default();

            // Split the borrows before creating the viewer
            let DialogueEditorState {
                snarl,
                id_mapping,
                next_node_id,
                snarl_node_ids,
                reverse_id_mapping,
                ..
            } = &mut *editor_state;

            // Create viewer with the split borrows
            let mut viewer = DialogueViewer {
                id_mapping,
                next_node_id,
                snarl_node_ids,
                reverse_id_mapping,
            };

            // Show the snarl editor
            snarl.show(&mut viewer, &style, egui::Id::new("dialogue_snarl"), ui);
        });
}

/// Save the current dialogue to a file
fn save_dialogue(editor_state: &DialogueEditorState, asset_server: &AssetServer) {
    // Create a dialogue graph from the editor state
    let mut graph = if let Some(start_node_id) = editor_state.id_mapping.values().next() {
        DialogueGraph::new(*start_node_id)
    } else {
        warn!("Cannot save empty dialogue");
        return;
    };

    graph = graph.with_name(&editor_state.current_dialogue_name);

    // Add all nodes to the graph
    for snarl_id in &editor_state.snarl_node_ids {
        let editor_node = &editor_state.snarl[*snarl_id];
        graph.add_node(editor_node.node.clone());
    }

    // Add all connections
    for &snarl_from_id in &editor_state.snarl_node_ids {
        let from_node = &editor_state.snarl[snarl_from_id];
        let from_node_id = from_node.id();

        // Get all output pins for this node
        let output_count = match &from_node.node {
            DialogueNode::Text { .. } => 1,
            DialogueNode::Choice { .. } => 3, // Default to 3 for now
        };

        for output_idx in 0..output_count {
            // Get the output pin
            let out_pin = editor_state.snarl.out_pin(egui_snarl::OutPinId {
                node: snarl_from_id,
                output: output_idx,
            });

            // Check if this output has a connection
            if let Some(in_pin) = out_pin.remotes.first() {
                let snarl_to_id = in_pin.node;
                if let Some(&to_node_id) = editor_state.id_mapping.get(&snarl_to_id) {
                    // Create connection with appropriate label for choice nodes
                    let label = match &from_node.node {
                        DialogueNode::Choice { .. } => Some(format!("Choice {}", output_idx + 1)),
                        _ => None,
                    };

                    if let Err(e) =
                        graph.connect(from_node_id, to_node_id, ConnectionData::new(label))
                    {
                        warn!("Failed to add connection: {}", e);
                    }
                }
            }
        }
    }

    // Create dialogue asset
    let asset = DialogueAsset::new(graph);

    // Serialize to JSON
    match serde_json::to_string_pretty(&asset) {
        Ok(json) => {
            // For now, just log it. In a real implementation, you'd save to a file
            info!("Dialogue JSON:\n{}", json);

            // TODO: Actually save to file system
            // This would require file dialog integration or a predefined save path
        }
        Err(e) => {
            error!("Failed to serialize dialogue: {}", e);
        }
    }
}
