/*
 * Early UI module - needs a lot of work, adapted from example.
 */
use bevy::prelude::*;
use funkus_dialogue_core::{DialogueAsset, DialogueNode, DialogueRunner, DialogueState};

use crate::components::*;

/// System to display dialogue content.
pub fn display_dialogue(
    mut commands: Commands,
    dialogue_assets: Res<Assets<DialogueAsset>>,
    dialogue_query: Query<(&DialogueRunner, &Name)>,
    mut speaker_query: Query<&mut Text, With<SpeakerText>>,
    mut dialogue_query_text: Query<
        &mut Text,
        (
            With<DialogueText>,
            Without<SpeakerText>,
            Without<ChoiceText>,
        ),
    >,
    choices_query: Query<Entity, With<ChoicesContainer>>,
) {
    // Find the first active dialogue
    for (runner, _) in dialogue_query.iter() {
        if runner.state == DialogueState::Inactive {
            // Clear UI when dialogue is inactive
            for mut speaker_text in speaker_query.iter_mut() {
                *speaker_text = Text::new("");
            }

            for mut dialogue_text in dialogue_query_text.iter_mut() {
                *dialogue_text = Text::new("");
            }

            for choices_entity in choices_query.iter() {
                commands.entity(choices_entity).despawn_descendants();
            }

            continue;
        }

        // Get dialogue asset
        if let Some(dialogue) = dialogue_assets.get(&runner.dialogue_handle) {
            if let Some(node_id) = runner.current_node_id {
                if let Some(node) = dialogue.graph.get_node(node_id) {
                    // Process based on node type
                    match node {
                        DialogueNode::Text { text, speaker, .. } => {
                            // Update speaker
                            for mut speaker_text in speaker_query.iter_mut() {
                                if let Some(speaker_name) = speaker {
                                    *speaker_text = Text::new(speaker_name.clone());
                                } else {
                                    *speaker_text = Text::new("");
                                }
                            }

                            // Update dialogue text
                            for mut dialogue_text in dialogue_query_text.iter_mut() {
                                *dialogue_text = Text::new(text.clone());
                            }

                            // Clear choices
                            for choices_entity in choices_query.iter() {
                                commands.entity(choices_entity).despawn_descendants();
                            }
                        }
                        DialogueNode::Choice {
                            prompt, speaker, ..
                        } => {
                            // Update speaker
                            for mut speaker_text in speaker_query.iter_mut() {
                                if let Some(speaker_name) = speaker {
                                    *speaker_text = Text::new(speaker_name.clone());
                                } else {
                                    *speaker_text = Text::new("");
                                }
                            }

                            // Update dialogue text (prompt)
                            for mut dialogue_text in dialogue_query_text.iter_mut() {
                                if let Some(prompt_text) = prompt {
                                    *dialogue_text = Text::new(prompt_text.clone());
                                } else {
                                    *dialogue_text = Text::new("Choose an option:");
                                }
                            }

                            // Handle the ChoiceSelected state
                            let selected_index = match runner.state {
                                DialogueState::ChoiceSelected(index) => Some(index),
                                _ => None,
                            };

                            // Get connections from the graph structure
                            let connections = dialogue.graph.get_connected_nodes(node_id);

                            for choices_entity in choices_query.iter() {
                                commands.entity(choices_entity).despawn_descendants();

                                // Add choice buttons
                                for (i, (_, label)) in connections.iter().enumerate() {
                                    let choice_text = label
                                        .clone()
                                        .unwrap_or_else(|| format!("Choice {}", i + 1));

                                    let display_text = if Some(i) == selected_index {
                                        // Highlight selected choice
                                        format!("▶ {}. {}", i + 1, choice_text)
                                    } else {
                                        format!("{}. {}", i + 1, choice_text)
                                    };

                                    commands.entity(choices_entity).with_children(|parent| {
                                        parent.spawn((
                                            Text::new(display_text),
                                            TextFont {
                                                font_size: 16.0,
                                                ..default()
                                            },
                                            TextColor(if Some(i) == selected_index {
                                                Color::srgb(1.0, 1.0, 0.5) // Highlight selected choice
                                            } else {
                                                Color::srgb(0.8, 0.8, 1.0)
                                            }),
                                            Node {
                                                margin: UiRect::bottom(Val::Px(5.0)),
                                                ..default()
                                            },
                                            ChoiceText(i),
                                        ));
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
