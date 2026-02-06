//! Node canvas rendering backed by egui-snarl.
//!
//! The snarl widget owns interaction state (selection, drag, wire edits). We mirror
//! its selection into `DialogueNodeEditorState` so the inspector can reflect it.

use std::collections::{HashMap, HashSet};

use bevy_egui::egui::{self, Color32, Pos2, RichText, Stroke, Ui, Vec2, pos2, vec2};
use egui_snarl::ui::{PinInfo, PinPlacement, SnarlPin, SnarlStyle, SnarlViewer, SnarlWidget};
use egui_snarl::{InPin, InPinId, NodeId as SnarlNodeId, OutPin, OutPinId, Snarl};
use funkus_dialogue_core::graph::{ConnectionData, DialogueGraph, DialogueNode, NodeId};

const NODE_GRID_COLUMNS: usize = 4;
const NODE_GRID_SPACING: Vec2 = vec2(280.0, 180.0);
const NODE_BODY_WIDTH: f32 = 220.0;
const NODE_BODY_MAX_CHARS: usize = 64;
const NODE_OUTPUT_MAX_CHARS: usize = 28;

const TEXT_NODE_COLOR: Color32 = Color32::from_rgb(0x4A, 0xB0, 0xE6);
const CHOICE_NODE_COLOR: Color32 = Color32::from_rgb(0xE6, 0x9D, 0x4A);
const ADD_SLOT_COLOR: Color32 = Color32::from_rgb(0x8A, 0x8A, 0x8A);
const START_NODE_COLOR: Color32 = Color32::from_rgb(0x2F, 0x5E, 0x3B);
const EFFECT_NODE_COLOR: Color32 = Color32::from_rgb(0x7A, 0x6A, 0xE6);

#[derive(Clone, Debug)]
pub struct DialogueNodeView {
    pub graph_id: NodeId,
}

#[derive(Clone, Debug)]
pub struct DialogueNodeEditorState {
    pub snarl: Snarl<DialogueNodeView>,
    graph_to_snarl: HashMap<NodeId, SnarlNodeId>,
    spawn_index: usize,
    pub selected_nodes: Vec<NodeId>,
    pending_selection: Option<NodeId>,
}

impl DialogueNodeEditorState {
    #[must_use]
    pub fn from_graph(graph: &DialogueGraph) -> Self {
        let mut state = Self {
            snarl: Snarl::new(),
            graph_to_snarl: HashMap::new(),
            spawn_index: 0,
            selected_nodes: Vec::new(),
            pending_selection: None,
        };
        state.rebuild_from_graph(graph);
        state
    }

    pub fn rebuild_from_graph(&mut self, graph: &DialogueGraph) {
        self.snarl = Snarl::new();
        self.graph_to_snarl.clear();
        self.selected_nodes.clear();
        self.pending_selection = None;

        let mut ids = graph.node_ids();
        ids.sort_by_key(|id| id.raw());

        for (index, id) in ids.iter().enumerate() {
            let pos = Self::grid_position(index);
            let snarl_id = self
                .snarl
                .insert_node(pos, DialogueNodeView { graph_id: *id });
            self.graph_to_snarl.insert(*id, snarl_id);
        }

        for from in ids {
            let connections = sorted_connections(graph, from);
            for (output_index, (to, _)) in connections.into_iter().enumerate() {
                if let (Some(&from_snarl), Some(&to_snarl)) =
                    (self.graph_to_snarl.get(&from), self.graph_to_snarl.get(&to))
                {
                    let out_pin = OutPinId {
                        node: from_snarl,
                        output: output_index,
                    };
                    let in_pin = InPinId {
                        node: to_snarl,
                        input: 0,
                    };
                    self.snarl.connect(out_pin, in_pin);
                }
            }
        }

        self.spawn_index = self.graph_to_snarl.len();
    }

    pub fn ensure_graph_sync(&mut self, graph: &DialogueGraph) {
        let graph_ids: HashSet<NodeId> = graph.node_ids().into_iter().collect();

        let mut stale = Vec::new();
        for id in self.graph_to_snarl.keys() {
            if !graph_ids.contains(id) {
                stale.push(*id);
            }
        }

        for id in stale {
            if let Some(snarl_id) = self.graph_to_snarl.remove(&id) {
                self.snarl.remove_node(snarl_id);
            }
        }

        for id in graph_ids {
            if !self.graph_to_snarl.contains_key(&id) {
                let pos = Self::next_spawn_pos(&mut self.spawn_index);
                let snarl_id = self
                    .snarl
                    .insert_node(pos, DialogueNodeView { graph_id: id });
                self.graph_to_snarl.insert(id, snarl_id);
            }
        }

        self.spawn_index = self.spawn_index.max(self.graph_to_snarl.len());
        self.selected_nodes
            .retain(|id| self.graph_to_snarl.contains_key(id));
    }

    pub fn refresh_connections_for_node(&mut self, graph: &DialogueGraph, node_id: NodeId) {
        let Some(&snarl_id) = self.graph_to_snarl.get(&node_id) else {
            return;
        };
        let Some(node) = graph.get_node(node_id) else {
            return;
        };

        let connections = graph.get_connected_nodes(node_id);
        let output_count = match node {
            DialogueNode::Text { .. } | DialogueNode::Effect { .. } => 1,
            DialogueNode::Choice { .. } => {
                let count = connections.len();
                if count == 0 { 1 } else { count + 1 }
            }
        };

        for output in 0..output_count {
            let out_pin = OutPinId {
                node: snarl_id,
                output,
            };
            self.snarl.drop_outputs(out_pin);
        }

        for (output_index, (target, _)) in connections.iter().enumerate() {
            if let Some(&to_snarl) = self.graph_to_snarl.get(target) {
                let out_pin = OutPinId {
                    node: snarl_id,
                    output: output_index,
                };
                let in_pin = InPinId {
                    node: to_snarl,
                    input: 0,
                };
                self.snarl.connect(out_pin, in_pin);
            }
        }
    }

    pub fn request_selection(&mut self, id: NodeId) {
        self.pending_selection = Some(id);
    }

    pub fn drop_selection(&mut self, id: NodeId) {
        self.selected_nodes.retain(|existing| *existing != id);
    }

    pub fn split_mut(
        &mut self,
    ) -> (
        &mut Snarl<DialogueNodeView>,
        &mut HashMap<NodeId, SnarlNodeId>,
        &mut usize,
    ) {
        (
            &mut self.snarl,
            &mut self.graph_to_snarl,
            &mut self.spawn_index,
        )
    }

    fn next_spawn_pos(spawn_index: &mut usize) -> Pos2 {
        let pos = Self::grid_position(*spawn_index);
        *spawn_index = spawn_index.saturating_add(1);
        pos
    }

    fn grid_position(index: usize) -> Pos2 {
        let column = index % NODE_GRID_COLUMNS;
        let row = index / NODE_GRID_COLUMNS;
        pos2(
            column as f32 * NODE_GRID_SPACING.x,
            row as f32 * NODE_GRID_SPACING.y,
        )
    }
}

pub fn draw_dialogue_node_editor(
    ui: &mut Ui,
    graph: &mut DialogueGraph,
    state: &mut DialogueNodeEditorState,
) -> bool {
    state.ensure_graph_sync(graph);
    let mut preserve_selection = false;
    if let Some(requested) = state.pending_selection.take() {
        state.selected_nodes = vec![requested];
        preserve_selection = true;
    }
    let mut add_text_node = false;
    let mut add_choice_node = false;
    let mut add_effect_node = false;

    ui.horizontal(|ui| {
        if ui.button("Add Text Node").clicked() {
            add_text_node = true;
        }

        if ui.button("Add Choice Node").clicked() {
            add_choice_node = true;
        }

        if ui.button("Add Effect Node").clicked() {
            add_effect_node = true;
        }
    });

    ui.separator();

    let mut selected = Vec::new();
    let mut dirty = false;
    let (viewer_dirty, response_clicked) = {
        let (snarl, graph_to_snarl, spawn_index) = state.split_mut();

        if add_text_node {
            spawn_node(
                graph,
                snarl,
                graph_to_snarl,
                spawn_index,
                DialogueNode::text("New dialogue line"),
            );
            dirty = true;
        }

        if add_choice_node {
            spawn_node(
                graph,
                snarl,
                graph_to_snarl,
                spawn_index,
                DialogueNode::choice(),
            );
            dirty = true;
        }

        if add_effect_node {
            spawn_node(
                graph,
                snarl,
                graph_to_snarl,
                spawn_index,
                DialogueNode::effect(funkus_dialogue_core::registry::DialogueEffect::set(
                    "game.flag",
                    funkus_dialogue_core::registry::DialogueValue::Bool(true),
                )),
            );
            dirty = true;
        }

        let mut viewer = DialogueSnarlViewer::new(graph, graph_to_snarl, spawn_index);
        let available = ui.available_size();
        let snarl_widget = SnarlWidget::new()
            .id_salt("dialogue_snarl")
            .min_size(available)
            .style(SnarlStyle {
                pin_placement: Some(PinPlacement::Edge),
                ..SnarlStyle::new()
            });
        let response = snarl_widget.show(snarl, &mut viewer, ui);
        let response_clicked = response.clicked();
        let viewer_dirty = viewer.dirty;

        for snarl_id in snarl_widget.get_selected_nodes(ui) {
            if let Some(view) = snarl.get_node(snarl_id) {
                selected.push(view.graph_id);
            }
        }
        (viewer_dirty, response_clicked)
    };

    // Selection comes from egui-snarl; we copy it into editor state for the inspector.
    if !selected.is_empty() {
        state.selected_nodes = selected;
    } else if response_clicked && !preserve_selection {
        state.selected_nodes.clear();
    }

    viewer_dirty || dirty
}

fn spawn_node(
    graph: &mut DialogueGraph,
    snarl: &mut Snarl<DialogueNodeView>,
    graph_to_snarl: &mut HashMap<NodeId, SnarlNodeId>,
    spawn_index: &mut usize,
    node: DialogueNode,
) -> NodeId {
    let node_id = graph.add_node(node);
    let pos = DialogueNodeEditorState::next_spawn_pos(spawn_index);
    let snarl_id = snarl.insert_node(pos, DialogueNodeView { graph_id: node_id });
    graph_to_snarl.insert(node_id, snarl_id);
    if graph.start_node.is_none() {
        let _ = graph.set_start_node(node_id);
    }
    node_id
}

struct DialogueSnarlViewer<'a> {
    graph: &'a mut DialogueGraph,
    graph_to_snarl: &'a mut HashMap<NodeId, SnarlNodeId>,
    spawn_index: &'a mut usize,
    dirty: bool,
}

impl<'a> DialogueSnarlViewer<'a> {
    fn new(
        graph: &'a mut DialogueGraph,
        graph_to_snarl: &'a mut HashMap<NodeId, SnarlNodeId>,
        spawn_index: &'a mut usize,
    ) -> Self {
        Self {
            graph,
            graph_to_snarl,
            spawn_index,
            dirty: false,
        }
    }

    fn graph_id_for_node(
        &self,
        node: SnarlNodeId,
        snarl: &Snarl<DialogueNodeView>,
    ) -> Option<NodeId> {
        snarl.get_node(node).map(|view| view.graph_id)
    }

    fn node_kind(&self, id: NodeId) -> Option<&DialogueNode> {
        self.graph.get_node(id)
    }

    fn output_count(&self, id: NodeId) -> usize {
        match self.node_kind(id) {
            Some(DialogueNode::Text { .. }) | Some(DialogueNode::Effect { .. }) => 1,
            Some(DialogueNode::Choice { .. }) => {
                let count = sorted_connections(self.graph, id).len();
                if count == 0 {
                    1
                } else {
                    count.saturating_add(1)
                }
            }
            None => 0,
        }
    }

    fn output_label(&self, id: NodeId, output_index: usize) -> OutputLabel {
        match self.node_kind(id) {
            Some(DialogueNode::Text { .. }) | Some(DialogueNode::Effect { .. }) => OutputLabel {
                short: "Next".to_string(),
                full: None,
                is_add_slot: false,
            },
            Some(DialogueNode::Choice { .. }) => {
                let connections = sorted_connections(self.graph, id);
                if output_index < connections.len() {
                    let full = connections[output_index]
                        .1
                        .clone()
                        .unwrap_or_else(|| format!("Choice {}", output_index + 1));
                    OutputLabel {
                        short: truncate_label(&full, NODE_OUTPUT_MAX_CHARS),
                        full: Some(full),
                        is_add_slot: false,
                    }
                } else {
                    OutputLabel {
                        short: "+ Choice".to_string(),
                        full: None,
                        is_add_slot: true,
                    }
                }
            }
            None => OutputLabel {
                short: "Missing".to_string(),
                full: None,
                is_add_slot: false,
            },
        }
    }

    fn add_snarl_node(
        &mut self,
        snarl: &mut Snarl<DialogueNodeView>,
        graph_id: NodeId,
        pos: Pos2,
    ) -> SnarlNodeId {
        let snarl_id = snarl.insert_node(pos, DialogueNodeView { graph_id });
        self.graph_to_snarl.insert(graph_id, snarl_id);
        snarl_id
    }

    fn remove_snarl_node(
        &mut self,
        snarl: &mut Snarl<DialogueNodeView>,
        snarl_id: SnarlNodeId,
    ) -> NodeId {
        let view = snarl.remove_node(snarl_id);
        self.graph_to_snarl.remove(&view.graph_id);
        view.graph_id
    }

    fn bump_spawn_index(&mut self) {
        *self.spawn_index = self.spawn_index.saturating_add(1);
    }
}

struct OutputLabel {
    short: String,
    full: Option<String>,
    is_add_slot: bool,
}

impl SnarlViewer<DialogueNodeView> for DialogueSnarlViewer<'_> {
    fn title(&mut self, node: &DialogueNodeView) -> String {
        match self.node_kind(node.graph_id) {
            Some(DialogueNode::Text { .. }) => format!("Text {}", node.graph_id.raw()),
            Some(DialogueNode::Choice { .. }) => format!("Choice {}", node.graph_id.raw()),
            Some(DialogueNode::Effect { .. }) => format!("Effect {}", node.graph_id.raw()),
            None => format!("Missing {}", node.graph_id.raw()),
        }
    }

    fn node_frame(
        &mut self,
        mut default: egui::Frame,
        node: SnarlNodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        snarl: &Snarl<DialogueNodeView>,
    ) -> egui::Frame {
        if let Some(graph_id) = self.graph_id_for_node(node, snarl) {
            if self.graph.start_node == Some(graph_id) {
                default.fill = START_NODE_COLOR;
                default.stroke = Stroke::new(1.5, Color32::from_rgb(0x6B, 0xC5, 0x7A));
            }
        }
        default
    }

    fn show_header(
        &mut self,
        node: SnarlNodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        ui: &mut Ui,
        snarl: &mut Snarl<DialogueNodeView>,
    ) {
        if let Some(graph_id) = self.graph_id_for_node(node, snarl) {
            ui.horizontal(|ui| {
                ui.label(self.title(&snarl[node]));
                if self.graph.start_node == Some(graph_id) {
                    ui.label(RichText::new("Start").color(Color32::LIGHT_GREEN));
                } else if ui.small_button("Set Start").clicked() {
                    let _ = self.graph.set_start_node(graph_id);
                    self.dirty = true;
                }
            });
        } else {
            ui.label("Missing node");
        }
    }

    fn inputs(&mut self, _node: &DialogueNodeView) -> usize {
        1
    }

    fn show_input(
        &mut self,
        pin: &InPin,
        ui: &mut Ui,
        snarl: &mut Snarl<DialogueNodeView>,
    ) -> impl SnarlPin + 'static {
        let graph_id = snarl[pin.id.node].graph_id;
        ui.label("In");
        if self.graph.start_node == Some(graph_id) {
            PinInfo::circle().with_fill(Color32::LIGHT_GREEN)
        } else {
            PinInfo::circle().with_fill(Color32::GRAY)
        }
    }

    fn outputs(&mut self, node: &DialogueNodeView) -> usize {
        self.output_count(node.graph_id)
    }

    fn show_output(
        &mut self,
        pin: &OutPin,
        ui: &mut Ui,
        snarl: &mut Snarl<DialogueNodeView>,
    ) -> impl SnarlPin + 'static {
        let graph_id = snarl[pin.id.node].graph_id;
        let output = self.output_label(graph_id, pin.id.output);
        let response = ui.label(output.short.clone());
        if let Some(full) = output.full.as_ref() {
            if *full != output.short {
                response.on_hover_text(full);
            }
        }

        match self.node_kind(graph_id) {
            Some(DialogueNode::Text { .. }) => PinInfo::triangle().with_fill(TEXT_NODE_COLOR),
            Some(DialogueNode::Choice { .. }) => {
                PinInfo::triangle().with_fill(if output.is_add_slot {
                    ADD_SLOT_COLOR
                } else {
                    CHOICE_NODE_COLOR
                })
            }
            Some(DialogueNode::Effect { .. }) => PinInfo::triangle().with_fill(EFFECT_NODE_COLOR),
            None => PinInfo::triangle().with_fill(Color32::DARK_GRAY),
        }
    }

    fn has_body(&mut self, _node: &DialogueNodeView) -> bool {
        true
    }

    fn show_body(
        &mut self,
        node: SnarlNodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        ui: &mut Ui,
        snarl: &mut Snarl<DialogueNodeView>,
    ) {
        let Some(graph_id) = self.graph_id_for_node(node, snarl) else {
            ui.label("Missing node data.");
            return;
        };

        let connections_len = sorted_connections(self.graph, graph_id).len();

        let Some(node_data) = self.graph.get_node_mut(graph_id) else {
            ui.label("Node not found in graph.");
            return;
        };

        match node_data {
            DialogueNode::Text { text, speaker, .. } => {
                ui.vertical(|ui| {
                    ui.set_width(NODE_BODY_WIDTH);
                    ui.label(RichText::new("Text Node").strong());
                    if let Some(speaker_name) = speaker.as_ref() {
                        ui.add_sized(
                            [NODE_BODY_WIDTH, 0.0],
                            egui::Label::new(
                                RichText::new(format!("Speaker: {speaker_name}")).small(),
                            )
                            .wrap(),
                        );
                    }
                    let preview = snippet(text, NODE_BODY_MAX_CHARS);
                    let response = ui.add_sized(
                        [NODE_BODY_WIDTH, 0.0],
                        egui::Label::new(preview.clone()).wrap(),
                    );
                    if preview != *text {
                        response.on_hover_text(text.as_str());
                    }
                });
            }
            DialogueNode::Choice {
                prompt, speaker, ..
            } => {
                ui.vertical(|ui| {
                    ui.set_width(NODE_BODY_WIDTH);
                    ui.label(RichText::new("Choice Node").strong());
                    if let Some(speaker_name) = speaker.as_ref() {
                        ui.add_sized(
                            [NODE_BODY_WIDTH, 0.0],
                            egui::Label::new(
                                RichText::new(format!("Speaker: {speaker_name}")).small(),
                            )
                            .wrap(),
                        );
                    }
                    if let Some(prompt_text) = prompt.as_ref() {
                        let preview = snippet(prompt_text, NODE_BODY_MAX_CHARS);
                        let response = ui.add_sized(
                            [NODE_BODY_WIDTH, 0.0],
                            egui::Label::new(preview.clone()).wrap(),
                        );
                        if preview != *prompt_text {
                            response.on_hover_text(prompt_text.as_str());
                        }
                    } else {
                        ui.add_sized(
                            [NODE_BODY_WIDTH, 0.0],
                            egui::Label::new(RichText::new("Prompt: (none)").small()).wrap(),
                        );
                    }
                    ui.label(RichText::new(format!("Outputs: {}", connections_len)).small());
                });
            }
            DialogueNode::Effect { effect } => {
                ui.vertical(|ui| {
                    ui.set_width(NODE_BODY_WIDTH);
                    ui.label(RichText::new("Effect Node").strong());
                    ui.add_sized(
                        [NODE_BODY_WIDTH, 0.0],
                        egui::Label::new(RichText::new(format!("Key: {}", effect.key)).small())
                            .wrap(),
                    );
                    ui.add_sized(
                        [NODE_BODY_WIDTH, 0.0],
                        egui::Label::new(RichText::new(format!("Op: {:?}", effect.op)).small())
                            .wrap(),
                    );
                });
            }
        }
    }

    fn has_node_menu(&mut self, _node: &DialogueNodeView) -> bool {
        true
    }

    fn show_node_menu(
        &mut self,
        node: SnarlNodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        ui: &mut Ui,
        snarl: &mut Snarl<DialogueNodeView>,
    ) {
        let Some(graph_id) = self.graph_id_for_node(node, snarl) else {
            ui.label("Missing node");
            return;
        };

        if ui.button("Set as Start Node").clicked() {
            let _ = self.graph.set_start_node(graph_id);
            self.dirty = true;
            ui.close();
        }

        if ui
            .add(egui::Button::new("Delete Node").fill(Color32::DARK_RED))
            .clicked()
        {
            let _ = self.graph.remove_node(graph_id);
            self.remove_snarl_node(snarl, node);
            self.dirty = true;
            ui.close();
        }
    }

    fn has_graph_menu(&mut self, _pos: Pos2, _snarl: &mut Snarl<DialogueNodeView>) -> bool {
        true
    }

    fn show_graph_menu(&mut self, pos: Pos2, ui: &mut Ui, snarl: &mut Snarl<DialogueNodeView>) {
        ui.label("Add node");
        if ui.button("Text").clicked() {
            let node_id = self.graph.add_node(DialogueNode::text("New dialogue line"));
            self.add_snarl_node(snarl, node_id, pos);
            self.bump_spawn_index();
            if self.graph.start_node.is_none() {
                let _ = self.graph.set_start_node(node_id);
            }
            self.dirty = true;
            ui.close();
        }
        if ui.button("Choice").clicked() {
            let node_id = self.graph.add_node(DialogueNode::choice());
            self.add_snarl_node(snarl, node_id, pos);
            self.bump_spawn_index();
            if self.graph.start_node.is_none() {
                let _ = self.graph.set_start_node(node_id);
            }
            self.dirty = true;
            ui.close();
        }
        if ui.button("Effect").clicked() {
            let node_id = self.graph.add_node(DialogueNode::effect(
                funkus_dialogue_core::registry::DialogueEffect::set(
                    "game.flag",
                    funkus_dialogue_core::registry::DialogueValue::Bool(true),
                ),
            ));
            self.add_snarl_node(snarl, node_id, pos);
            self.bump_spawn_index();
            if self.graph.start_node.is_none() {
                let _ = self.graph.set_start_node(node_id);
            }
            self.dirty = true;
            ui.close();
        }
    }

    fn connect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<DialogueNodeView>) {
        let (Some(from_graph), Some(to_graph)) = (
            self.graph_id_for_node(from.id.node, snarl),
            self.graph_id_for_node(to.id.node, snarl),
        ) else {
            return;
        };

        if from_graph == to_graph {
            return;
        }

        let is_text_node = matches!(
            self.node_kind(from_graph),
            Some(DialogueNode::Text { .. } | DialogueNode::Effect { .. })
        );

        if is_text_node {
            let connections = sorted_connections(self.graph, from_graph);
            for (target, _) in connections {
                let _ = self.graph.disconnect(from_graph, target);
            }
            for remote in &from.remotes {
                snarl.disconnect(from.id, *remote);
            }
        } else if let Some(existing_target) =
            connection_target_for_output(self.graph, from_graph, from.id.output)
        {
            let _ = self.graph.disconnect(from_graph, existing_target);
            for remote in &from.remotes {
                snarl.disconnect(from.id, *remote);
            }
        }

        let label = match self.node_kind(from_graph) {
            Some(DialogueNode::Choice { .. }) => {
                connection_label_for_output(self.graph, from_graph, from.id.output)
                    .or_else(|| Some(format!("Choice {}", from.id.output + 1)))
            }
            _ => None,
        };

        let _ = self
            .graph
            .connect(from_graph, to_graph, ConnectionData::new(label));
        snarl.connect(from.id, to.id);
        self.dirty = true;
    }

    fn disconnect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<DialogueNodeView>) {
        if let (Some(from_graph), Some(to_graph)) = (
            self.graph_id_for_node(from.id.node, snarl),
            self.graph_id_for_node(to.id.node, snarl),
        ) {
            let _ = self.graph.disconnect(from_graph, to_graph);
        }
        snarl.disconnect(from.id, to.id);
        self.dirty = true;
    }

    fn drop_outputs(&mut self, pin: &OutPin, snarl: &mut Snarl<DialogueNodeView>) {
        let Some(from_graph) = self.graph_id_for_node(pin.id.node, snarl) else {
            return;
        };

        for remote in &pin.remotes {
            if let Some(to_graph) = self.graph_id_for_node(remote.node, snarl) {
                let _ = self.graph.disconnect(from_graph, to_graph);
            }
        }
        snarl.drop_outputs(pin.id);
        self.dirty = true;
    }

    fn drop_inputs(&mut self, pin: &InPin, snarl: &mut Snarl<DialogueNodeView>) {
        let Some(to_graph) = self.graph_id_for_node(pin.id.node, snarl) else {
            return;
        };

        for remote in &pin.remotes {
            if let Some(from_graph) = self.graph_id_for_node(remote.node, snarl) {
                let _ = self.graph.disconnect(from_graph, to_graph);
            }
        }
        snarl.drop_inputs(pin.id);
        self.dirty = true;
    }

    fn final_node_rect(
        &mut self,
        _node: SnarlNodeId,
        _rect: egui::Rect,
        _ui: &mut Ui,
        _snarl: &mut Snarl<DialogueNodeView>,
    ) {
    }
}

fn sorted_connections(graph: &DialogueGraph, id: NodeId) -> Vec<(NodeId, Option<String>)> {
    graph.get_connected_nodes(id)
}

fn connection_target_for_output(
    graph: &DialogueGraph,
    id: NodeId,
    output_index: usize,
) -> Option<NodeId> {
    let connections = sorted_connections(graph, id);
    connections.get(output_index).map(|(target, _)| *target)
}

fn connection_label_for_output(
    graph: &DialogueGraph,
    id: NodeId,
    output_index: usize,
) -> Option<String> {
    let connections = sorted_connections(graph, id);
    connections
        .get(output_index)
        .and_then(|(_, label)| label.clone())
}

fn snippet(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        let mut shortened = text[..max_len].to_string();
        shortened.push_str("...");
        shortened
    }
}

fn truncate_label(text: &str, max_len: usize) -> String {
    snippet(text, max_len)
}
