//! Node body widget for the graph canvas.
//!
//! The body renderer delegates node-specific preview rendering to dedicated widgets
//! so each variant can evolve independently.

mod body_choice;
mod body_effect;
mod body_message;
mod body_text;

use bevy_egui::egui::Ui;
use funkus_dialogue_core::graph::DialogueNode;

use self::body_choice::ChoiceBodyWidget;
use self::body_effect::EffectBodyWidget;
use self::body_message::MessageBodyWidget;
use self::body_text::TextBodyWidget;

/// Input data used to render a node body.
pub struct NodeBodyData<'a> {
    /// Node content to preview.
    pub node: &'a DialogueNode,
    /// Number of outgoing connections for this node.
    pub connections_len: usize,
    /// Target width for body rows.
    pub body_width: f32,
    /// Maximum preview length for text/prompt snippets.
    pub body_max_chars: usize,
}

/// Small, focused renderer for node body summaries.
#[derive(Default)]
pub struct NodeBodyWidget {
    text: TextBodyWidget,
    choice: ChoiceBodyWidget,
    effect: EffectBodyWidget,
    message: MessageBodyWidget,
}

impl NodeBodyWidget {
    /// Draws the node body summary.
    pub fn show(&mut self, ui: &mut Ui, data: NodeBodyData<'_>) {
        match data.node {
            DialogueNode::Text { text, speaker, .. } => self.text.show(
                ui,
                text,
                speaker.as_deref(),
                data.body_width,
                data.body_max_chars,
            ),
            DialogueNode::Choice {
                prompt, speaker, ..
            } => self.choice.show(
                ui,
                prompt.as_deref(),
                speaker.as_deref(),
                data.connections_len,
                data.body_width,
                data.body_max_chars,
            ),
            DialogueNode::Effect { effect } => self.effect.show(ui, effect, data.body_width),
            DialogueNode::Message { message } => self.message.show(ui, message, data.body_width),
        }
    }
}

pub(crate) fn snippet(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        let split = text
            .char_indices()
            .nth(max_len)
            .map_or(text.len(), |(idx, _)| idx);
        let mut shortened = text[..split].to_string();
        shortened.push_str("...");
        shortened
    }
}
