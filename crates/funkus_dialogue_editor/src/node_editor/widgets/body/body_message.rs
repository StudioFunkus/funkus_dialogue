//! Message-node body widget.

use bevy_egui::egui::{self, RichText, Ui};
use funkus_dialogue_core::registry::DialogueMessageCall;

/// Renders summary content for message nodes.
#[derive(Default)]
pub struct MessageBodyWidget;

impl MessageBodyWidget {
    /// Draws message key and parameter count summary.
    pub fn show(&mut self, ui: &mut Ui, message: &DialogueMessageCall, body_width: f32) {
        ui.vertical(|ui| {
            ui.set_width(body_width);
            ui.add_sized(
                [body_width, 0.0],
                egui::Label::new(RichText::new(format!("Key: {}", message.key)).small()).wrap(),
            );
            ui.add_sized(
                [body_width, 0.0],
                egui::Label::new(
                    RichText::new(format!("Params: {}", message.params.len())).small(),
                )
                .wrap(),
            );
        });
    }
}
