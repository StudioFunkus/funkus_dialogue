//! Choice-node body widget.

use bevy_egui::egui::{self, RichText, Ui};

use super::snippet;

/// Renders summary content for choice nodes.
#[derive(Default)]
pub struct ChoiceBodyWidget;

impl ChoiceBodyWidget {
    /// Draws speaker, prompt preview, and output count for a choice node.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        prompt: Option<&str>,
        speaker: Option<&str>,
        connections_len: usize,
        body_width: f32,
        body_max_chars: usize,
    ) {
        ui.vertical(|ui| {
            ui.set_width(body_width);
            if let Some(speaker_name) = speaker {
                ui.add_sized(
                    [body_width, 0.0],
                    egui::Label::new(RichText::new(format!("Speaker: {speaker_name}")).small())
                        .wrap(),
                );
            }

            if let Some(prompt_text) = prompt {
                let preview = snippet(prompt_text, body_max_chars);
                let response =
                    ui.add_sized([body_width, 0.0], egui::Label::new(preview.clone()).wrap());
                if preview != prompt_text {
                    response.on_hover_text(prompt_text);
                }
            } else {
                ui.add_sized(
                    [body_width, 0.0],
                    egui::Label::new(RichText::new("Prompt: (none)").small()).wrap(),
                );
            }

            ui.label(RichText::new(format!("Outputs: {connections_len}")).small());
        });
    }
}
