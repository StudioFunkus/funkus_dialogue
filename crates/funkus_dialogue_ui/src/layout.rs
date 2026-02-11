use bevy::prelude::*;

use crate::components::{
    ChoicesContainer, DialogueDisplay, DialogueText, PortraitImage, SpeakerText,
};

const DIALOGUE_BOX_BOTTOM: f32 = 60.0;
const DIALOGUE_BOX_LEFT: f32 = 100.0;
const DIALOGUE_BOX_RIGHT: f32 = 100.0;
const DIALOGUE_BOX_HEIGHT: f32 = 200.0;
const CHOICES_GAP: f32 = 14.0;
const PANEL_PADDING: f32 = 10.0;
const PANEL_BORDER: f32 = 2.0;
const PANEL_BACKGROUND: Color = Color::srgba(0.1, 0.1, 0.1, 0.8);

/// Bundle for adding dialogue UI components to an entity.
#[derive(Bundle)]
pub struct DialogueUIBundle {
    pub display: DialogueDisplay,
}

/// Spawns the default dialogue UI entity tree.
pub fn spawn_dialogue_ui(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                right: Val::Px(0.0),
                bottom: Val::Px(0.0),
                left: Val::Px(0.0),
                ..default()
            },
            DialogueDisplay,
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        bottom: Val::Px(DIALOGUE_BOX_BOTTOM),
                        left: Val::Px(DIALOGUE_BOX_LEFT),
                        right: Val::Px(DIALOGUE_BOX_RIGHT),
                        height: Val::Px(DIALOGUE_BOX_HEIGHT),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::FlexStart,
                        padding: UiRect::all(Val::Px(PANEL_PADDING)),
                        border: UiRect::all(Val::Px(PANEL_BORDER)),
                        ..default()
                    },
                    BackgroundColor(PANEL_BACKGROUND),
                ))
                .with_children(|panel| {
                    // Portrait image (optional), anchored on the left side.
                    panel.spawn((
                        Node {
                            width: Val::Px(64.0),
                            height: Val::Px(64.0),
                            margin: UiRect::right(Val::Px(12.0)),
                            ..default()
                        },
                        ImageNode::default(),
                        PortraitImage,
                    ));

                    panel
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            flex_grow: 1.0,
                            min_width: Val::Px(0.0),
                            ..default()
                        })
                        .with_children(|text_column| {
                            // Speaker name.
                            text_column.spawn((
                                Text::new(""),
                                TextFont {
                                    font_size: 20.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                                SpeakerText,
                            ));

                            // Dialogue body text.
                            text_column.spawn((
                                Text::new(""),
                                TextFont {
                                    font_size: 18.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                                Node {
                                    margin: UiRect::top(Val::Px(10.0)),
                                    ..default()
                                },
                                DialogueText,
                            ));
                        });
                });

            // Choices container rendered in a dedicated panel above the dialogue box.
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(DIALOGUE_BOX_LEFT),
                    right: Val::Px(DIALOGUE_BOX_RIGHT),
                    bottom: Val::Px(DIALOGUE_BOX_BOTTOM + DIALOGUE_BOX_HEIGHT + CHOICES_GAP),
                    display: Display::None,
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(PANEL_PADDING)),
                    border: UiRect::all(Val::Px(PANEL_BORDER)),
                    ..default()
                },
                BackgroundColor(PANEL_BACKGROUND),
                ChoicesContainer,
            ));
        })
        .id()
}
