//! Node header widget for the graph canvas.

use bevy_egui::egui::{Align, Button, Color32, Layout, RichText, Ui};

use crate::node_editor::theme::GraphTheme;

/// Input data used to render a node header.
pub struct NodeHeaderData<'a> {
    /// Fully formatted node title.
    pub title: &'a str,
    /// True when this node is currently the start node.
    pub is_start: bool,
}

/// Actions emitted by the header widget.
#[derive(Default)]
pub struct NodeHeaderOutput {
    /// True when the user requests this node to become the start node.
    pub request_set_start: bool,
}

/// Small, focused renderer for node headers.
#[derive(Default)]
pub struct NodeHeaderWidget;

impl NodeHeaderWidget {
    /// Draws the header and returns user intent.
    #[must_use]
    pub fn show(&mut self, ui: &mut Ui, data: NodeHeaderData<'_>) -> NodeHeaderOutput {
        let mut output = NodeHeaderOutput::default();

        ui.horizontal(|ui| {
            ui.label(
                RichText::new(data.title)
                    .color(Color32::WHITE)
                    .strong()
                    .text_style(GraphTheme::text_style_node_header()),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add(Button::new("Set Start").small().selected(data.is_start))
                    .clicked()
                {
                    output.request_set_start = true;
                }
            });
        });

        output
    }
}
