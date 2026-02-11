use std::collections::HashMap;

use bevy::prelude::*;
use bevy::ui::auto_directional_navigation::AutoDirectionalNavigation;
use funkus_dialogue_core::{DialogueAsset, DialogueNode, DialogueRunner, DialogueState};

use crate::components::{ChoiceText, ChoicesContainer, DialogueText, PortraitImage, SpeakerText};
use crate::presentation::{ChoicePresentationMode, resolve_choice_presentation};

#[derive(Resource, Default)]
pub(crate) struct DialogueUiChoiceRenderCache {
    fingerprints: HashMap<Entity, ChoiceRenderFingerprint>,
}

#[derive(Clone, PartialEq, Eq)]
struct ChoiceRenderFingerprint {
    presentation: ChoicePresentationMode,
    labels: Vec<String>,
}

/// System to display dialogue content.
pub fn display_dialogue(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    dialogue_assets: Res<Assets<DialogueAsset>>,
    mut choice_render_cache: ResMut<DialogueUiChoiceRenderCache>,
    dialogue_runner_query: Query<(Entity, &DialogueRunner)>,
    choices_children_query: Query<&Children, With<ChoicesContainer>>,
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
    mut choice_item_query: Query<
        (&ChoiceText, &mut Text, &mut TextColor, &mut Node),
        (
            With<ChoiceText>,
            Without<ChoicesContainer>,
            Without<PortraitImage>,
            Without<SpeakerText>,
            Without<DialogueText>,
        ),
    >,
) {
    choice_render_cache
        .fingerprints
        .retain(|entity, _| choices_children_query.get(*entity).is_ok());

    // This default UI is single-view; if multiple dialogues are active, we render one
    // deterministic target so concurrent runners do not fight over the same widgets.
    let active_runner = dialogue_runner_query
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
            &choices_children_query,
            &mut choice_render_cache,
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
            &choices_children_query,
            &mut choice_render_cache,
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
            &choices_children_query,
            &mut choice_render_cache,
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
            &choices_children_query,
            &mut choice_render_cache,
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
                clear_choices_container(
                    &mut commands,
                    choices_entity,
                    &mut choices_node,
                    &choices_children_query,
                    &mut choice_render_cache,
                );
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

            let connections = dialogue.graph.get_outgoing_connections(node_id);
            let presentation = resolve_choice_presentation(presentation_key.as_deref());
            let labels: Vec<String> = connections
                .iter()
                .enumerate()
                .map(|(i, (_, data))| {
                    data.label
                        .clone()
                        .unwrap_or_else(|| format!("Choice {}", i + 1))
                })
                .collect();
            let selected_index = runner.clamped_selected_choice_index(labels.len());
            let fingerprint = ChoiceRenderFingerprint {
                presentation,
                labels: labels.clone(),
            };

            for (choices_entity, mut choices_node) in &mut choices_query {
                match presentation {
                    ChoicePresentationMode::StandardList => {
                        choices_node.display = Display::Flex;
                        choices_node.flex_direction = FlexDirection::Column;
                        choices_node.flex_wrap = FlexWrap::NoWrap;
                    }
                    ChoicePresentationMode::InlineBadges => {
                        choices_node.display = Display::Flex;
                        choices_node.flex_direction = FlexDirection::Row;
                        choices_node.flex_wrap = FlexWrap::Wrap;
                    }
                    ChoicePresentationMode::External => {
                        // Unknown/custom presentation modes are handled by game-specific UI.
                        // Hide the default choice container entirely.
                        clear_choices_container(
                            &mut commands,
                            choices_entity,
                            &mut choices_node,
                            &choices_children_query,
                            &mut choice_render_cache,
                        );
                        continue;
                    }
                }

                let children_count = choices_children_query
                    .get(choices_entity)
                    .map_or(0, |children| children.len());
                let cached = choice_render_cache.fingerprints.get(&choices_entity);
                let rebuild_children =
                    cached != Some(&fingerprint) || children_count != labels.len();

                if rebuild_children {
                    if children_count > 0 {
                        commands
                            .entity(choices_entity)
                            .despawn_related::<Children>();
                    }
                    spawn_choice_children(
                        &mut commands,
                        choices_entity,
                        presentation,
                        &labels,
                        selected_index,
                    );
                    choice_render_cache
                        .fingerprints
                        .insert(choices_entity, fingerprint.clone());
                    continue;
                }

                let Ok(children) = choices_children_query.get(choices_entity) else {
                    continue;
                };

                for child in children.iter() {
                    if let Ok((choice_index, mut choice_text, mut choice_color, mut choice_node)) =
                        choice_item_query.get_mut(child)
                    {
                        let Some(choice_label) = labels.get(choice_index.0) else {
                            commands.entity(child).despawn();
                            continue;
                        };

                        let (display_text, display_color, margin) = choice_visuals(
                            presentation,
                            choice_index.0,
                            choice_label,
                            Some(choice_index.0) == selected_index,
                        );
                        *choice_text = Text::new(display_text);
                        *choice_color = TextColor(display_color);
                        choice_node.margin = margin;
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
                &choices_children_query,
                &mut choice_render_cache,
            );
        }
    }
}

fn choice_visuals(
    presentation: ChoicePresentationMode,
    index: usize,
    label: &str,
    selected: bool,
) -> (String, Color, UiRect) {
    match presentation {
        ChoicePresentationMode::StandardList => {
            let text = if selected {
                format!("> {}. {}", index + 1, label)
            } else {
                format!("{}. {}", index + 1, label)
            };
            let color = if selected {
                Color::srgb(1.0, 1.0, 0.5)
            } else {
                Color::srgb(0.8, 0.8, 1.0)
            };
            (text, color, UiRect::bottom(Val::Px(5.0)))
        }
        ChoicePresentationMode::InlineBadges => {
            let text = if selected {
                format!("[ {} ]", label.to_uppercase())
            } else {
                format!("[ {} ]", label)
            };
            let color = if selected {
                Color::srgb(0.3, 1.0, 0.7)
            } else {
                Color::srgb(0.85, 0.85, 0.9)
            };
            (
                text,
                color,
                UiRect::new(Val::Px(0.0), Val::Px(8.0), Val::Px(4.0), Val::Px(4.0)),
            )
        }
        ChoicePresentationMode::External => (
            String::new(),
            Color::WHITE,
            UiRect::new(Val::Px(0.0), Val::Px(0.0), Val::Px(0.0), Val::Px(0.0)),
        ),
    }
}

fn spawn_choice_children(
    commands: &mut Commands,
    choices_entity: Entity,
    presentation: ChoicePresentationMode,
    labels: &[String],
    selected_index: Option<usize>,
) {
    for (i, label) in labels.iter().enumerate() {
        let (display_text, display_color, margin) =
            choice_visuals(presentation, i, label, Some(i) == selected_index);
        commands.entity(choices_entity).with_children(|parent| {
            let mut choice_entity = parent.spawn((
                Text::new(display_text),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(display_color),
                Node {
                    margin,
                    ..default()
                },
                ChoiceText(i),
            ));
            if matches!(presentation, ChoicePresentationMode::InlineBadges) {
                choice_entity.insert(AutoDirectionalNavigation::default());
            }
        });
    }
}

fn clear_choices_container(
    commands: &mut Commands,
    choices_entity: Entity,
    choices_node: &mut Node,
    choices_children_query: &Query<&Children, With<ChoicesContainer>>,
    choice_render_cache: &mut DialogueUiChoiceRenderCache,
) {
    choices_node.display = Display::None;
    choices_node.flex_direction = FlexDirection::Column;
    choices_node.flex_wrap = FlexWrap::NoWrap;
    choice_render_cache.fingerprints.remove(&choices_entity);

    if choices_children_query
        .get(choices_entity)
        .is_ok_and(|children| !children.is_empty())
    {
        commands
            .entity(choices_entity)
            .despawn_related::<Children>();
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
    choices_children_query: &Query<&Children, With<ChoicesContainer>>,
    choice_render_cache: &mut DialogueUiChoiceRenderCache,
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
        clear_choices_container(
            commands,
            choices_entity,
            &mut choices_node,
            choices_children_query,
            choice_render_cache,
        );
    }
}
