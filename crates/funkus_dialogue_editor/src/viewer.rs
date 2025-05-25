use bevy::prelude::*;
use bevy_egui::egui;
use egui_snarl::{
    InPin, NodeId as SnarlNodeId, OutPin, Snarl,
    ui::{PinInfo, SnarlViewer},
};
use funkus_dialogue_core::graph::{DialogueNode, NodeId};
use std::collections::HashMap;

use crate::node_wrapper::EditorNode;

/// The viewer implementation for the dialogue editor
pub struct DialogueViewer<'a> {
    pub id_mapping: &'a mut HashMap<SnarlNodeId, NodeId>,
    pub next_node_id: &'a mut u32,
    pub snarl_node_ids: &'a mut Vec<SnarlNodeId>,
    pub reverse_id_mapping: &'a mut HashMap<NodeId, SnarlNodeId>,
}

impl<'a> SnarlViewer<EditorNode> for DialogueViewer<'a> {
    fn title(&mut self, node: &EditorNode) -> String {
        match &node.node {
            DialogueNode::Text { speaker, text, .. } => {
                if let Some(speaker_name) = speaker {
                    format!("{}: {}", speaker_name, text.chars().take(20).collect::<String>())
                } else {
                    text.chars().take(30).collect::<String>()
                }
            }
            DialogueNode::Choice { speaker, prompt, .. } => {
                if let Some(speaker_name) = speaker {
                    format!("{}: Choice", speaker_name)
                } else if let Some(prompt_text) = prompt {
                    format!("Choice: {}", prompt_text.chars().take(20).collect::<String>())
                } else {
                    "Choice Node".to_string()
                }
            }
        }
    }

    fn inputs(&mut self, _node: &EditorNode) -> usize {
        1 // All nodes have one input
    }

    fn outputs(&mut self, node: &EditorNode) -> usize {
        match &node.node {
            DialogueNode::Text { .. } => 1,
            DialogueNode::Choice { .. } => {
                // For now, we'll default to 3 outputs for choice nodes
                // In a real implementation, this would be based on actual connections
                3
            }
        }
    }

    #[allow(refining_impl_trait)]
    fn show_input(
        &mut self,
        _pin: &InPin,
        ui: &mut egui::Ui,
        _scale: f32,
        _snarl: &mut Snarl<EditorNode>,
    ) -> PinInfo {
        ui.label("→");
        PinInfo::circle().with_fill(egui::Color32::from_rgb(100, 100, 100))
    }

    #[allow(refining_impl_trait)]
    fn show_output(
        &mut self,
        pin: &OutPin,
        ui: &mut egui::Ui,
        _scale: f32,
        snarl: &mut Snarl<EditorNode>,
    ) -> PinInfo {
        let node = &snarl[pin.id.node];
        match &node.node {
            DialogueNode::Text { .. } => {
                ui.label("→");
            }
            DialogueNode::Choice { .. } => {
                ui.label(format!("Choice {}", pin.id.output + 1));
            }
        }
        PinInfo::circle().with_fill(egui::Color32::from_rgb(100, 150, 100))
    }

    fn connect(
        &mut self,
        from: &OutPin,
        to: &InPin,
        snarl: &mut Snarl<EditorNode>,
    ) {
        // Remove existing connection to this input
        for &remote in &to.remotes.clone() {
            snarl.disconnect(remote, to.id);
        }
        snarl.connect(from.id, to.id);
    }

    fn disconnect(
        &mut self,
        from: &OutPin,
        to: &InPin,
        snarl: &mut Snarl<EditorNode>,
    ) {
        snarl.disconnect(from.id, to.id);
    }

    fn has_node_menu(&mut self, _node: &EditorNode) -> bool {
        true
    }

    fn show_node_menu(
        &mut self,
        node: SnarlNodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        ui: &mut egui::Ui,
        _scale: f32,
        snarl: &mut Snarl<EditorNode>,
    ) {
        ui.label("Node Actions");
        
        let editor_node = &mut snarl[node];
        
        match &mut editor_node.node {
            DialogueNode::Text { text, speaker, portrait, .. } => {
                ui.vertical(|ui| {
                    ui.label("Text Node");
                    ui.separator();
                    
                    ui.horizontal(|ui| {
                        ui.label("Speaker:");
                        let mut speaker_text = speaker.clone().unwrap_or_default();
                        let response = ui.text_edit_singleline(&mut speaker_text);
                        if response.changed() {
                            *speaker = if speaker_text.is_empty() { None } else { Some(speaker_text) };
                        }
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label("Portrait:");
                        let mut portrait_text = portrait.clone().unwrap_or_default();
                        let response = ui.text_edit_singleline(&mut portrait_text);
                        if response.changed() {
                            *portrait = if portrait_text.is_empty() { None } else { Some(portrait_text) };
                        }
                    });
                    
                    ui.label("Text:");
                    ui.text_edit_multiline(text);
                });
            }
            DialogueNode::Choice { prompt, speaker, portrait, .. } => {
                ui.vertical(|ui| {
                    ui.label("Choice Node");
                    ui.separator();
                    
                    ui.horizontal(|ui| {
                        ui.label("Speaker:");
                        let mut speaker_text = speaker.clone().unwrap_or_default();
                        let response = ui.text_edit_singleline(&mut speaker_text);
                        if response.changed() {
                            *speaker = if speaker_text.is_empty() { None } else { Some(speaker_text) };
                        }
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label("Portrait:");
                        let mut portrait_text = portrait.clone().unwrap_or_default();
                        let response = ui.text_edit_singleline(&mut portrait_text);
                        if response.changed() {
                            *portrait = if portrait_text.is_empty() { None } else { Some(portrait_text) };
                        }
                    });
                    
                    ui.label("Prompt:");
                    let mut prompt_text = prompt.clone().unwrap_or_default();
                    let response = ui.text_edit_multiline(&mut prompt_text);
                    if response.changed() {
                        *prompt = if prompt_text.is_empty() { None } else { Some(prompt_text) };
                    }
                });
            }
        }
        
        ui.separator();
        
        if ui.button("Remove Node").clicked() {
            // Find and remove from our tracking
            if let Some(&node_id) = self.id_mapping.get(&node) {
                self.reverse_id_mapping.remove(&node_id);
            }
            self.snarl_node_ids.retain(|&id| id != node);
            
            snarl.remove_node(node);
            ui.close_menu();
        }
    }

    fn has_graph_menu(&mut self, _pos: egui::Pos2, _snarl: &mut Snarl<EditorNode>) -> bool {
        true
    }

    fn show_graph_menu(
        &mut self,
        pos: egui::Pos2,
        ui: &mut egui::Ui,
        _scale: f32,
        snarl: &mut Snarl<EditorNode>,
    ) {
        ui.label("Add Node");
        ui.separator();
        
        if ui.button("Text Node").clicked() {
            let new_id = NodeId(*self.next_node_id);
            *self.next_node_id += 1;
            
            let node = DialogueNode::text(new_id, "New text node");
            let editor_node = EditorNode::new(node, (pos.x, pos.y));
            let snarl_id = snarl.insert_node(pos, editor_node);
            
            self.id_mapping.insert(snarl_id, new_id);
            self.reverse_id_mapping.insert(new_id, snarl_id);
            self.snarl_node_ids.push(snarl_id);
            
            ui.close_menu();
        }
        
        if ui.button("Choice Node").clicked() {
            let new_id = NodeId(*self.next_node_id);
            *self.next_node_id += 1;
            
            let node = DialogueNode::choice(new_id);
            let editor_node = EditorNode::new(node, (pos.x, pos.y));
            let snarl_id = snarl.insert_node(pos, editor_node);
            
            self.id_mapping.insert(snarl_id, new_id);
            self.reverse_id_mapping.insert(new_id, snarl_id);
            self.snarl_node_ids.push(snarl_id);
            
            ui.close_menu();
        }
    }
}