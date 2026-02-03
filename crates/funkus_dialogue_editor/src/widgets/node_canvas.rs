use bevy_egui::egui::Ui;

use funkus_dialogue_core::graph::DialogueGraph;

use crate::node_editor::DialogueNodeEditorState;
use crate::node_editor::draw_dialogue_node_editor;

pub struct NodeCanvasWidget;

pub struct NodeCanvasOutput {
    pub dirty: bool,
}

impl NodeCanvasWidget {
    pub fn show(
        &mut self,
        ui: &mut Ui,
        graph: &mut DialogueGraph,
        state: &mut DialogueNodeEditorState,
    ) -> NodeCanvasOutput {
        let dirty = draw_dialogue_node_editor(ui, graph, state);
        NodeCanvasOutput { dirty }
    }
}
