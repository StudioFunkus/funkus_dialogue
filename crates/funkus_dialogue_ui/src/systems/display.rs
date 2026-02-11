use bevy::prelude::*;
use funkus_dialogue_core::{DialogueAsset, DialogueNode, DialogueRunner, DialogueState};

use crate::components::{ChoiceText, ChoicesContainer, DialogueText, PortraitImage, SpeakerText};
use crate::presentation::{ChoicePresentationMode, resolve_choice_presentation};

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
    mut portrait_query: Query<
        (&mut ImageNode, &mut Node),
        (With<PortraitImage>, Without<ChoicesContainer>),
    >,
    mut choices_query: Query<(Entity, &mut Node), (With<ChoicesContainer>, Without<PortraitImage>)>,
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
            &mut choices_query,
        );
        return;
    };

    let Some(dialogue) = dialogue_assets.get(&runner.dialogue_handle) else {
        clear_dialogue_ui(
            &mut commands,
            &mut speaker_query,
            &mut dialogue_query_text,
            &mut portrait_query,
            &mut choices_query,
        );
        return;
    };

    let Some(node_id) = runner.current_node_id else {
        clear_dialogue_ui(
            &mut commands,
            &mut speaker_query,
            &mut dialogue_query_text,
            &mut portrait_query,
            &mut choices_query,
        );
        return;
    };

    let Some(node) = dialogue.graph.get_node(node_id) else {
        clear_dialogue_ui(
            &mut commands,
            &mut speaker_query,
            &mut dialogue_query_text,
            &mut portrait_query,
            &mut choices_query,
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

            for (choices_entity, mut choices_node) in &mut choices_query {
                choices_node.flex_direction = FlexDirection::Column;
                choices_node.flex_wrap = FlexWrap::NoWrap;
                commands
                    .entity(choices_entity)
                    .despawn_related::<Children>();
            }
        }
        DialogueNode::Choice {
            prompt,
            presentation_key,
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

            let connections = dialogue.graph.get_outgoing_connections(node_id);
            let presentation = resolve_choice_presentation(presentation_key.as_deref());

            for (choices_entity, mut choices_node) in &mut choices_query {
                commands
                    .entity(choices_entity)
                    .despawn_related::<Children>();

                match presentation {
                    ChoicePresentationMode::StandardList => {
                        choices_node.display = Display::Flex;
                        choices_node.flex_direction = FlexDirection::Column;
                        choices_node.flex_wrap = FlexWrap::NoWrap;

                        for (i, (_, data)) in connections.iter().enumerate() {
                            let choice_text = data
                                .label
                                .clone()
                                .unwrap_or_else(|| format!("Choice {}", i + 1));

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
                    ChoicePresentationMode::InlineBadges => {
                        choices_node.display = Display::Flex;
                        choices_node.flex_direction = FlexDirection::Row;
                        choices_node.flex_wrap = FlexWrap::Wrap;

                        for (i, (_, data)) in connections.iter().enumerate() {
                            let choice_text = data
                                .label
                                .clone()
                                .unwrap_or_else(|| format!("Choice {}", i + 1));
                            let display_text = if Some(i) == selected_index {
                                format!("[ {} ]", choice_text.to_uppercase())
                            } else {
                                format!("[ {} ]", choice_text)
                            };

                            commands.entity(choices_entity).with_children(|parent| {
                                parent.spawn((
                                    Text::new(display_text),
                                    TextFont {
                                        font_size: 16.0,
                                        ..default()
                                    },
                                    TextColor(if Some(i) == selected_index {
                                        Color::srgb(0.3, 1.0, 0.7)
                                    } else {
                                        Color::srgb(0.85, 0.85, 0.9)
                                    }),
                                    Node {
                                        margin: UiRect::new(
                                            Val::Px(0.0),
                                            Val::Px(8.0),
                                            Val::Px(4.0),
                                            Val::Px(4.0),
                                        ),
                                        ..default()
                                    },
                                    ChoiceText(i),
                                ));
                            });
                        }
                    }
                    ChoicePresentationMode::External => {
                        // Unknown/custom presentation modes are handled by game-specific UI.
                        // Hide the default choice container entirely.
                        choices_node.display = Display::None;
                        choices_node.flex_direction = FlexDirection::Column;
                        choices_node.flex_wrap = FlexWrap::NoWrap;
                    }
                }
            }
        }
        DialogueNode::Effect { .. } | DialogueNode::Message { .. } => {
            clear_dialogue_ui(
                &mut commands,
                &mut speaker_query,
                &mut dialogue_query_text,
                &mut portrait_query,
                &mut choices_query,
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
    portrait_query: &mut Query<
        (&mut ImageNode, &mut Node),
        (With<PortraitImage>, Without<ChoicesContainer>),
    >,
    choices_query: &mut Query<
        (Entity, &mut Node),
        (With<ChoicesContainer>, Without<PortraitImage>),
    >,
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

    for (choices_entity, mut choices_node) in choices_query.iter_mut() {
        choices_node.display = Display::Flex;
        choices_node.flex_direction = FlexDirection::Column;
        choices_node.flex_wrap = FlexWrap::NoWrap;
        commands
            .entity(choices_entity)
            .despawn_related::<Children>();
    }
}
