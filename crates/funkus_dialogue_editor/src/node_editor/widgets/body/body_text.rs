//! Text-node body widget.

use bevy_egui::egui::{self, RichText, Ui};

use super::snippet;

/// Renders summary content for text nodes.
#[derive(Default)]
pub struct TextBodyWidget;

impl TextBodyWidget {
    /// Draws speaker and text preview for a text node.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        text: &str,
        speaker: Option<&str>,
        body_width: f32,
        body_max_chars: usize,
    ) {
        ui.vertical(|ui| {
            ui.set_width(body_width);
            ui.label(RichText::new("Text Node").strong());
            if let Some(speaker_name) = speaker {
                ui.add_sized(
                    [body_width, 0.0],
                    egui::Label::new(RichText::new(format!("Speaker: {speaker_name}")).small())
                        .wrap(),
                );
            }

            let preview = snippet(text, body_max_chars);
            let response =
                ui.add_sized([body_width, 0.0], egui::Label::new(preview.clone()).wrap());
            if preview != text {
                response.on_hover_text(text);
            }
        });
    }
}
