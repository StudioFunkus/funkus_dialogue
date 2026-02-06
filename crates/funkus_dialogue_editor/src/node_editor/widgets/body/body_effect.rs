//! Effect-node body widget.

use bevy_egui::egui::{self, RichText, Ui};
use funkus_dialogue_core::registry::DialogueEffect;

/// Renders summary content for effect nodes.
#[derive(Default)]
pub struct EffectBodyWidget;

impl EffectBodyWidget {
    /// Draws registry key and operation summary for an effect node.
    pub fn show(&mut self, ui: &mut Ui, effect: &DialogueEffect, body_width: f32) {
        ui.vertical(|ui| {
            ui.set_width(body_width);
            ui.add_sized(
                [body_width, 0.0],
                egui::Label::new(RichText::new(format!("Key: {}", effect.key)).small()).wrap(),
            );
            ui.add_sized(
                [body_width, 0.0],
                egui::Label::new(RichText::new(format!("Op: {:?}", effect.op)).small()).wrap(),
            );
        });
    }
}
