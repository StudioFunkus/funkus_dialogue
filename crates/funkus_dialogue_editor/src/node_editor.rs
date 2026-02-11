//! Node canvas rendering backed by egui-snarl.
//!
//! The snarl widget owns interaction state (selection, drag, wire edits). We mirror
//! its selection into `DialogueNodeEditorState` so the inspector can reflect it.

mod theme;
mod widgets;

use std::collections::{HashMap, HashSet};

use bevy_egui::egui::{self, Color32, Pos2, Ui, Vec2, pos2, vec2};
use egui_snarl::ui::{
    BackgroundPattern, Grid, PinInfo, PinPlacement, SnarlPin, SnarlStyle, SnarlViewer, SnarlWidget,
};
use egui_snarl::{InPin, InPinId, NodeId as SnarlNodeId, OutPin, OutPinId, Snarl};
use funkus_dialogue_core::graph::{ConnectionData, DialogueGraph, DialogueNode, NodeId};
use funkus_dialogue_core::{DialogueEditorMetadata, DialogueEditorNodeMetadata};
use theme::GraphTheme;
use widgets::{NodeBodyData, NodeBodyWidget, NodeHeaderData, NodeHeaderWidget};

const NODE_GRID_COLUMNS: usize = 4;
const NODE_GRID_SPACING: Vec2 = vec2(280.0, 180.0);
const NODE_BODY_WIDTH: f32 = 220.0;
const NODE_BODY_MAX_CHARS: usize = 64;
const NODE_OUTPUT_MAX_CHARS: usize = 28;

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
    canvas_selected_nodes: Vec<NodeId>,
    selection_override: Option<Vec<NodeId>>,
}

impl DialogueNodeEditorState {
    #[must_use]
    pub fn from_graph(graph: &DialogueGraph) -> Self {
        Self::from_graph_with_layout(graph, None)
    }

    /// Builds editor interaction state from a dialogue graph, applying persisted layout metadata
    /// if available.
    #[must_use]
    pub fn from_graph_with_layout(
        graph: &DialogueGraph,
        layout: Option<&DialogueEditorMetadata>,
    ) -> Self {
        let mut state = Self {
            snarl: Snarl::new(),
            graph_to_snarl: HashMap::new(),
            spawn_index: 0,
            selected_nodes: Vec::new(),
            canvas_selected_nodes: Vec::new(),
            selection_override: None,
        };
        state.rebuild_from_graph(graph, layout);
        state
    }

    pub fn rebuild_from_graph(
        &mut self,
        graph: &DialogueGraph,
        layout: Option<&DialogueEditorMetadata>,
    ) {
        self.snarl = Snarl::new();
        self.graph_to_snarl.clear();
        self.selected_nodes.clear();
        self.canvas_selected_nodes.clear();
        self.selection_override = None;

        let mut ids = graph.node_ids();
        ids.sort_by_key(|id| id.raw());

        for (index, id) in ids.iter().enumerate() {
            let entry = layout.and_then(|layout| layout.node(*id)).copied();
            let (pos, collapsed) = entry.map_or_else(
                || (Self::grid_position(index), false),
                |entry| (pos2(entry.pos[0], entry.pos[1]), entry.collapsed),
            );

            let snarl_id = if collapsed {
                self.snarl
                    .insert_node_collapsed(pos, DialogueNodeView { graph_id: *id })
            } else {
                self.snarl
                    .insert_node(pos, DialogueNodeView { graph_id: *id })
            };
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
        self.canvas_selected_nodes
            .retain(|id| self.graph_to_snarl.contains_key(id));
        if let Some(override_selection) = &mut self.selection_override {
            override_selection.retain(|id| self.graph_to_snarl.contains_key(id));
            if override_selection.is_empty() {
                self.selection_override = None;
            }
        }
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
            DialogueNode::Text { .. }
            | DialogueNode::Effect { .. }
            | DialogueNode::Message { .. } => 1,
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
        self.selected_nodes = vec![id];
        self.selection_override = Some(vec![id]);
    }

    pub fn drop_selection(&mut self, id: NodeId) {
        self.selected_nodes.retain(|existing| *existing != id);
        self.canvas_selected_nodes
            .retain(|existing| *existing != id);
        if let Some(override_selection) = &mut self.selection_override {
            override_selection.retain(|existing| *existing != id);
            if override_selection.is_empty() {
                self.selection_override = None;
            }
        }
    }

    /// Builds tooling-only metadata describing node layout (position + collapsed state).
    #[must_use]
    pub fn editor_metadata(&self) -> DialogueEditorMetadata {
        let mut nodes: Vec<DialogueEditorNodeMetadata> = self
            .graph_to_snarl
            .iter()
            .filter_map(|(&graph_id, &snarl_id)| {
                let info = self.snarl.get_node_info(snarl_id)?;
                Some(DialogueEditorNodeMetadata {
                    id: graph_id,
                    pos: [info.pos.x, info.pos.y],
                    collapsed: !info.open,
                })
            })
            .collect();
        nodes.sort_by_key(|entry| entry.id.raw());
        DialogueEditorMetadata { nodes }
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
    let mut add_text_node = false;
    let mut add_choice_node = false;
    let mut add_effect_node = false;
    let mut add_message_node = false;

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

        if ui.button("Add Message Node").clicked() {
            add_message_node = true;
        }
    });

    ui.separator();

    let mut canvas_selected = Vec::new();
    let mut dirty = false;
    let viewer_dirty = {
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

        if add_message_node {
            spawn_node(
                graph,
                snarl,
                graph_to_snarl,
                spawn_index,
                DialogueNode::message(funkus_dialogue_core::registry::DialogueMessageCall::new(
                    "game.message",
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
                bg_pattern: Some(BackgroundPattern::Grid(Grid::new(
                    GraphTheme::GRID_SPACING,
                    0.0,
                ))),
                bg_pattern_stroke: Some(GraphTheme::grid_stroke()),
                collapsible: Some(true),
                pin_placement: Some(PinPlacement::Edge),
                wire_width: Some(GraphTheme::WIRE_WIDTH),
                ..SnarlStyle::new()
            });

        // Apply node-canvas typography without affecting the surrounding editor UI.
        let old_style = ui.style().clone();
        GraphTheme::apply_node_text_styles(ui.style_mut());
        let response = snarl_widget.show(snarl, &mut viewer, ui);
        *ui.style_mut() = old_style.as_ref().clone();
        let _ = response;
        let viewer_dirty = viewer.dirty;

        for snarl_id in snarl_widget.get_selected_nodes(ui) {
            if let Some(view) = snarl.get_node(snarl_id) {
                canvas_selected.push(view.graph_id);
            }
        }
        viewer_dirty
    };

    // Selection from egui-snarl is the source of truth for canvas interaction.
    // Always mirror it (including empty) so modifier-based deselection does not go stale.
    canvas_selected.sort_unstable_by_key(|id| id.raw());
    reconcile_effective_selection(
        &mut state.canvas_selected_nodes,
        &mut state.selection_override,
        &mut state.selected_nodes,
        canvas_selected,
    );

    viewer_dirty || dirty
}

fn reconcile_effective_selection(
    canvas_selected_nodes: &mut Vec<NodeId>,
    selection_override: &mut Option<Vec<NodeId>>,
    selected_nodes: &mut Vec<NodeId>,
    canvas_selected: Vec<NodeId>,
) {
    let canvas_changed = canvas_selected != *canvas_selected_nodes;
    *canvas_selected_nodes = canvas_selected.clone();

    // Inspector-driven selection override persists until canvas selection changes.
    if canvas_changed {
        *selection_override = None;
    }

    if let Some(override_selection) = selection_override.clone() {
        *selected_nodes = override_selection;
    } else {
        *selected_nodes = canvas_selected;
    }
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
    header_widget: NodeHeaderWidget,
    body_widget: NodeBodyWidget,
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
            header_widget: NodeHeaderWidget,
            body_widget: NodeBodyWidget::default(),
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
            Some(DialogueNode::Text { .. })
            | Some(DialogueNode::Effect { .. })
            | Some(DialogueNode::Message { .. }) => 1,
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
            Some(DialogueNode::Text { .. })
            | Some(DialogueNode::Effect { .. })
            | Some(DialogueNode::Message { .. }) => OutputLabel {
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
            Some(DialogueNode::Text { .. }) => {
                format!("Node #{} (Text)", node.graph_id.raw())
            }
            Some(DialogueNode::Choice { .. }) => {
                format!("Node #{} (Choice)", node.graph_id.raw())
            }
            Some(DialogueNode::Effect { .. }) => {
                format!("Node #{} (Effect)", node.graph_id.raw())
            }
            Some(DialogueNode::Message { .. }) => {
                format!("Node #{} (Message)", node.graph_id.raw())
            }
            None => format!("Node #{} (Missing)", node.graph_id.raw()),
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
                // Highlight the start node with an outline.
                default.stroke = GraphTheme::start_node_stroke();
            }
        }
        default
    }

    fn header_frame(
        &mut self,
        mut default: egui::Frame,
        node: SnarlNodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        snarl: &Snarl<DialogueNodeView>,
    ) -> egui::Frame {
        if let Some(graph_id) = self.graph_id_for_node(node, snarl)
            && let Some(node) = self.node_kind(graph_id)
        {
            default.fill = GraphTheme::palette_for_node(node).header_fill;
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
            let title = self.title(&snarl[node]);
            let output = self.header_widget.show(
                ui,
                NodeHeaderData {
                    title: &title,
                    is_start: self.graph.start_node == Some(graph_id),
                },
            );
            if output.request_set_start {
                let _ = self.graph.set_start_node(graph_id);
                self.dirty = true;
            }
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
        let wire_color = self.infer_input_wire_color(pin, snarl);
        let graph_id = snarl[pin.id.node].graph_id;
        ui.label("In");
        let mut info = PinInfo::circle().with_fill(GraphTheme::INPUT_PIN_FILL);
        if let Some(wire_color) = wire_color {
            info = info.with_wire_color(wire_color);
        }
        if self.graph.start_node == Some(graph_id) {
            info = info.with_stroke(GraphTheme::start_node_stroke());
        }
        info
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

        let mut info = PinInfo::triangle();
        if output.is_add_slot {
            info = info
                .with_fill(GraphTheme::ADD_SLOT_LINK)
                .with_wire_color(GraphTheme::ADD_SLOT_LINK);
            return info;
        }

        match self.node_kind(graph_id) {
            Some(node) => {
                let palette = GraphTheme::palette_for_node(node);
                info.with_fill(palette.link_color)
                    .with_wire_color(palette.link_color)
            }
            None => info.with_fill(Color32::DARK_GRAY),
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

        let Some(node_data) = self.graph.get_node(graph_id) else {
            ui.label("Node not found in graph.");
            return;
        };

        self.body_widget.show(
            ui,
            NodeBodyData {
                node: node_data,
                connections_len,
                body_width: NODE_BODY_WIDTH,
                body_max_chars: NODE_BODY_MAX_CHARS,
            },
        );
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
        if ui.button("Message").clicked() {
            let node_id = self.graph.add_node(DialogueNode::message(
                funkus_dialogue_core::registry::DialogueMessageCall::new("game.message"),
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
            Some(
                DialogueNode::Text { .. }
                    | DialogueNode::Effect { .. }
                    | DialogueNode::Message { .. }
            )
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

impl DialogueSnarlViewer<'_> {
    fn infer_input_wire_color(
        &self,
        pin: &InPin,
        snarl: &Snarl<DialogueNodeView>,
    ) -> Option<Color32> {
        let mut remotes = pin.remotes.iter();
        let first = remotes.next()?;
        let mut color = self.out_pin_wire_color(*first, snarl)?;
        for remote in remotes {
            if let Some(next) = self.out_pin_wire_color(*remote, snarl) {
                color = Color32::from_rgba_premultiplied(
                    u8::midpoint(color.r(), next.r()),
                    u8::midpoint(color.g(), next.g()),
                    u8::midpoint(color.b(), next.b()),
                    u8::midpoint(color.a(), next.a()),
                );
            }
        }
        Some(color)
    }

    fn out_pin_wire_color(
        &self,
        pin: OutPinId,
        snarl: &Snarl<DialogueNodeView>,
    ) -> Option<Color32> {
        let graph_id = snarl.get_node(pin.node)?.graph_id;
        let node = self.node_kind(graph_id)?;
        Some(GraphTheme::palette_for_node(node).link_color)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_metadata_is_applied_when_building_snarl() {
        let mut graph = DialogueGraph::new();
        let node_id = graph.add_node(DialogueNode::text("Hello"));

        let layout = DialogueEditorMetadata {
            nodes: vec![DialogueEditorNodeMetadata {
                id: node_id,
                pos: [100.0, 200.0],
                collapsed: true,
            }],
        };

        let state = DialogueNodeEditorState::from_graph_with_layout(&graph, Some(&layout));

        let snarl_id = state
            .snarl
            .node_ids()
            .find_map(|(snarl_id, view)| (view.graph_id == node_id).then_some(snarl_id))
            .expect("snarl node exists for graph node");

        let info = state
            .snarl
            .get_node_info(snarl_id)
            .expect("snarl node info exists");

        assert_eq!(info.pos.x, 100.0);
        assert_eq!(info.pos.y, 200.0);
        assert!(!info.open, "collapsed nodes should be inserted closed");
    }

    #[test]
    fn editor_metadata_is_extracted_from_snarl_state() {
        let mut graph = DialogueGraph::new();
        let node_id = graph.add_node(DialogueNode::text("Hello"));
        let mut state = DialogueNodeEditorState::from_graph(&graph);

        let snarl_id = state
            .snarl
            .node_ids()
            .find_map(|(snarl_id, view)| (view.graph_id == node_id).then_some(snarl_id))
            .expect("snarl node exists for graph node");

        let info = state
            .snarl
            .get_node_info_mut(snarl_id)
            .expect("snarl node info exists");
        info.pos = pos2(42.0, 77.0);
        info.open = true;

        let meta = state.editor_metadata();
        let entry = meta
            .node(node_id)
            .copied()
            .expect("metadata contains entry for node");

        assert_eq!(entry.pos, [42.0, 77.0]);
        assert!(!entry.collapsed);
    }

    #[test]
    fn reconcile_selection_mirrors_empty_canvas_selection() {
        let mut canvas_selected_nodes = vec![NodeId::from_raw(1)];
        let mut selection_override = None;
        let mut selected_nodes = vec![NodeId::from_raw(1)];

        reconcile_effective_selection(
            &mut canvas_selected_nodes,
            &mut selection_override,
            &mut selected_nodes,
            Vec::new(),
        );

        assert!(selected_nodes.is_empty());
        assert!(canvas_selected_nodes.is_empty());
    }

    #[test]
    fn reconcile_selection_keeps_override_until_canvas_changes() {
        let mut canvas_selected_nodes = vec![NodeId::from_raw(1)];
        let mut selection_override = Some(vec![NodeId::from_raw(5)]);
        let mut selected_nodes = vec![NodeId::from_raw(5)];

        reconcile_effective_selection(
            &mut canvas_selected_nodes,
            &mut selection_override,
            &mut selected_nodes,
            vec![NodeId::from_raw(1)],
        );

        assert_eq!(selected_nodes, vec![NodeId::from_raw(5)]);
        assert_eq!(selection_override, Some(vec![NodeId::from_raw(5)]));

        reconcile_effective_selection(
            &mut canvas_selected_nodes,
            &mut selection_override,
            &mut selected_nodes,
            vec![NodeId::from_raw(2)],
        );

        assert_eq!(selected_nodes, vec![NodeId::from_raw(2)]);
        assert!(selection_override.is_none());
    }
}
