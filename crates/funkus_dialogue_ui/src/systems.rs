/*
 * Early UI module - needs a lot of work, adapted from example.
 */
use bevy::prelude::*;
use funkus_dialogue_core::{DialogueAsset, DialogueNode, DialogueRunner, DialogueState};

use crate::components::*;

/// System to display dialogue content.
pub fn display_dialogue(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    dialogue_assets: Res<Assets<DialogueAsset>>,
    dialogue_query: Query<(Entity, &DialogueRunner)>,
    mut speaker_query: Query<&mut Text, With<SpeakerText>>,
    mut dialogue_query_text: Query<
        &mut Text,
        (
            With<DialogueText>,
            Without<SpeakerText>,
            Without<ChoiceText>,
        ),
    >,
    mut portrait_query: Query<(&mut ImageNode, &mut Node), With<PortraitImage>>,
    choices_query: Query<Entity, With<ChoicesContainer>>,
) {
    // This default UI is single-view; if multiple dialogues are active, we render one
    // deterministic target so concurrent runners do not fight over the same widgets.
    let active_runner = dialogue_query
        .iter()
        .filter(|(_, runner)| runner.state != DialogueState::Inactive)
        .min_by_key(|(entity, _)| entity.to_bits())
        .map(|(_, runner)| runner);

    let Some(runner) = active_runner else {
        clear_dialogue_ui(
            &mut commands,
            &mut speaker_query,
            &mut dialogue_query_text,
            &mut portrait_query,
            &choices_query,
        );
        return;
    };

    let Some(dialogue) = dialogue_assets.get(&runner.dialogue_handle) else {
        clear_dialogue_ui(
            &mut commands,
            &mut speaker_query,
            &mut dialogue_query_text,
            &mut portrait_query,
            &choices_query,
        );
        return;
    };

    let Some(node_id) = runner.current_node_id else {
        clear_dialogue_ui(
            &mut commands,
            &mut speaker_query,
            &mut dialogue_query_text,
            &mut portrait_query,
            &choices_query,
        );
        return;
    };

    let Some(node) = dialogue.graph.get_node(node_id) else {
        clear_dialogue_ui(
            &mut commands,
            &mut speaker_query,
            &mut dialogue_query_text,
            &mut portrait_query,
            &choices_query,
        );
        return;
    };

    match node {
        DialogueNode::Text {
            text,
            speaker,
            portrait,
            ..
        } => {
            for mut speaker_text in speaker_query.iter_mut() {
                if let Some(speaker_name) = speaker {
                    *speaker_text = Text::new(speaker_name.clone());
                } else {
                    *speaker_text = Text::new("");
                }
            }

            for mut dialogue_text in dialogue_query_text.iter_mut() {
                *dialogue_text = Text::new(text.clone());
            }

            for (mut portrait_image, mut node) in portrait_query.iter_mut() {
                if let Some(path) = portrait.as_ref() {
                    portrait_image.image = asset_server.load(path.clone());
                    node.display = Display::Flex;
                } else {
                    node.display = Display::None;
                }
            }

            for choices_entity in choices_query.iter() {
                commands
                    .entity(choices_entity)
                    .despawn_related::<Children>();
            }
        }
        DialogueNode::Choice {
            prompt,
            speaker,
            portrait,
            ..
        } => {
            for mut speaker_text in speaker_query.iter_mut() {
                if let Some(speaker_name) = speaker {
                    *speaker_text = Text::new(speaker_name.clone());
                } else {
                    *speaker_text = Text::new("");
                }
            }

            for mut dialogue_text in dialogue_query_text.iter_mut() {
                if let Some(prompt_text) = prompt {
                    *dialogue_text = Text::new(prompt_text.clone());
                } else {
                    *dialogue_text = Text::new("Choose an option:");
                }
            }

            for (mut portrait_image, mut node) in portrait_query.iter_mut() {
                if let Some(path) = portrait.as_ref() {
                    portrait_image.image = asset_server.load(path.clone());
                    node.display = Display::Flex;
                } else {
                    node.display = Display::None;
                }
            }

            let selected_index = match runner.state {
                DialogueState::ChoiceSelected(index) => Some(index),
                _ => None,
            };

            let connections = dialogue.graph.get_connected_nodes(node_id);

            for choices_entity in choices_query.iter() {
                commands
                    .entity(choices_entity)
                    .despawn_related::<Children>();

                for (i, (_, label)) in connections.iter().enumerate() {
                    let choice_text = label.clone().unwrap_or_else(|| format!("Choice {}", i + 1));

                    let display_text = if Some(i) == selected_index {
                        format!("> {}. {}", i + 1, choice_text)
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
                                Color::srgb(1.0, 1.0, 0.5)
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
        DialogueNode::Effect { .. } | DialogueNode::Message { .. } => {
            clear_dialogue_ui(
                &mut commands,
                &mut speaker_query,
                &mut dialogue_query_text,
                &mut portrait_query,
                &choices_query,
            );
        }
    }
}

fn clear_dialogue_ui(
    commands: &mut Commands,
    speaker_query: &mut Query<&mut Text, With<SpeakerText>>,
    dialogue_query_text: &mut Query<
        &mut Text,
        (
            With<DialogueText>,
            Without<SpeakerText>,
            Without<ChoiceText>,
        ),
    >,
    portrait_query: &mut Query<(&mut ImageNode, &mut Node), With<PortraitImage>>,
    choices_query: &Query<Entity, With<ChoicesContainer>>,
) {
    for mut speaker_text in speaker_query.iter_mut() {
        *speaker_text = Text::new("");
    }

    for mut dialogue_text in dialogue_query_text.iter_mut() {
        *dialogue_text = Text::new("");
    }

    for (mut portrait_image, mut node) in portrait_query.iter_mut() {
        portrait_image.image = default();
        node.display = Display::None;
    }

    for choices_entity in choices_query.iter() {
        commands
            .entity(choices_entity)
            .despawn_related::<Children>();
    }
}
