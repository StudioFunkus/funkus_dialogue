//! Visual theme for the dialogue graph editor.
//!
//! Keep all "what color should this be?" decisions in one place so the editor can evolve
//! without scattering style constants across the rendering code.

use bevy_egui::egui::{Color32, FontFamily, FontId, Stroke, Style, TextStyle, Vec2, vec2};

use funkus_dialogue_core::graph::DialogueNode;

/// Shared visual configuration for the node canvas.
///
/// This is intentionally minimal: it covers what we currently need (header colors, link colors,
/// grid styling, typography (font sizes), and start-node outline).
pub struct GraphTheme;

impl GraphTheme {
    /// Grid spacing in canvas coordinates.
    pub const GRID_SPACING: Vec2 = vec2(48.0, 48.0);

    /// Stroke used to render the canvas background grid.
    #[must_use]
    pub fn grid_stroke() -> Stroke {
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(160, 160, 160, 40))
    }

    /// Default base font size for node content in points/pixels.
    ///
    /// This is applied via a scoped egui [`Style`] around the node canvas so the rest of the
    /// editor UI is unaffected.
    pub const FONT_BODY_SIZE: f32 = 16.0;

    /// Smaller font size for secondary/meta labels inside nodes.
    pub const FONT_META_SIZE: f32 = 13.0;

    /// Slightly larger font size used for node titles.
    pub const FONT_HEADER_SIZE: f32 = 18.0;

    /// "CSS class" for node title text.
    ///
    /// Egui doesn't have CSS, but named [`TextStyle`] entries provide similar reuse.
    #[must_use]
    pub fn text_style_node_header() -> TextStyle {
        TextStyle::Name("fd_node_header".into())
    }

    /// Applies the node canvas typography "classes" to a [`Style`].
    ///
    /// Call this in a `ui.scope(...)` around the snarl widget. This keeps the rest of the editor
    /// (left panel, inspector, etc) using the global theme.
    pub fn apply_node_text_styles(style: &mut Style) {
        let family = style
            .text_styles
            .get(&TextStyle::Body)
            .map(|font_id| font_id.family.clone())
            .unwrap_or(FontFamily::Proportional);

        style.text_styles.insert(
            TextStyle::Body,
            FontId::new(Self::FONT_BODY_SIZE, family.clone()),
        );
        style.text_styles.insert(
            TextStyle::Button,
            FontId::new(Self::FONT_BODY_SIZE, family.clone()),
        );
        style.text_styles.insert(
            TextStyle::Small,
            FontId::new(Self::FONT_META_SIZE, family.clone()),
        );
        style.text_styles.insert(
            Self::text_style_node_header(),
            FontId::new(Self::FONT_HEADER_SIZE, family),
        );
    }

    /// Default wire thickness for connections.
    pub const WIRE_WIDTH: f32 = 2.0;

    /// Input pin fill (pins are used for interaction, wires for semantics).
    pub const INPUT_PIN_FILL: Color32 = Color32::from_gray(170);

    /// Start-node outline stroke.
    #[must_use]
    pub fn start_node_stroke() -> Stroke {
        Stroke::new(2.0, Color32::from_rgb(0x6B, 0xC5, 0x7A))
    }

    /// Header + link palette for the text node type.
    pub const TEXT: NodePalette = NodePalette {
        header_fill: Color32::from_rgb(0x2A, 0x79, 0xA6),
        link_color: Color32::from_rgb(0x4A, 0xB0, 0xE6),
    };

    /// Header + link palette for the choice node type.
    pub const CHOICE: NodePalette = NodePalette {
        header_fill: Color32::from_rgb(0xA6, 0x6F, 0x2A),
        link_color: Color32::from_rgb(0xE6, 0x9D, 0x4A),
    };

    /// Header + link palette for the effect node type.
    pub const EFFECT: NodePalette = NodePalette {
        header_fill: Color32::from_rgb(0x54, 0x48, 0xB0),
        link_color: Color32::from_rgb(0x7A, 0x6A, 0xE6),
    };

    /// Header + link palette for the message node type.
    pub const MESSAGE: NodePalette = NodePalette {
        header_fill: Color32::from_rgb(0x2A, 0x86, 0x6A),
        link_color: Color32::from_rgb(0x42, 0xC0, 0x98),
    };

    /// Link color used for non-semantic "add slot" pins on choice nodes.
    pub const ADD_SLOT_LINK: Color32 = Color32::from_rgb(0x8A, 0x8A, 0x8A);

    /// Returns the palette for a dialogue node.
    pub fn palette_for_node(node: &DialogueNode) -> NodePalette {
        match node {
            DialogueNode::Text { .. } => Self::TEXT,
            DialogueNode::Choice { .. } => Self::CHOICE,
            DialogueNode::Effect { .. } => Self::EFFECT,
            DialogueNode::Message { .. } => Self::MESSAGE,
        }
    }
}

/// Colors that define a node type's identity in the graph editor.
#[derive(Clone, Copy, Debug)]
pub struct NodePalette {
    /// Background fill for the node header.
    pub header_fill: Color32,
    /// Color used for wires leaving the node.
    pub link_color: Color32,
}
