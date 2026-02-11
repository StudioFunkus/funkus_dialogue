use bevy::prelude::*;
use bevy::{math::CompassOctant, ui::auto_directional_navigation::AutoDirectionalNavigator};
use funkus_dialogue_core::{
    AdvanceDialogue, DialogueAsset, DialogueRunner, DialogueState, SelectDialogueChoice,
};

use crate::components::{ChoiceText, ChoicesContainer};
use crate::presentation::{ChoicePresentationMode, resolve_choice_presentation};

/// Keyboard controls for the default dialogue UI.
///
/// This system intentionally ignores unknown external presentation keys so game-specific UIs
/// can provide their own input behavior.
pub fn default_choice_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    dialogue_assets: Res<Assets<DialogueAsset>>,
    dialogue_query: Query<(Entity, &DialogueRunner)>,
    choices_container_query: Query<(Entity, &Node, &Children), With<ChoicesContainer>>,
    choice_entity_query: Query<(Entity, &ChoiceText), With<ChoiceText>>,
    mut auto_directional_navigator: Option<AutoDirectionalNavigator>,
    mut select_events: MessageWriter<SelectDialogueChoice>,
    mut advance_events: MessageWriter<AdvanceDialogue>,
) {
    let Some((entity, runner)) = dialogue_query
        .iter()
        .filter(|(_, runner)| runner.state != DialogueState::Inactive)
        .min_by_key(|(entity, _)| entity.to_bits())
    else {
        return;
    };

    let confirm_pressed = keyboard.just_pressed(KeyCode::Enter)
        || keyboard.just_pressed(KeyCode::NumpadEnter)
        || keyboard.just_pressed(KeyCode::Space);

    if runner.state == DialogueState::ShowingText {
        if confirm_pressed {
            advance_events.write(AdvanceDialogue { entity });
        }
        return;
    }

    if !runner.state.can_select_choice() {
        return;
    }

    let Some(dialogue) = dialogue_assets.get(&runner.dialogue_handle) else {
        return;
    };

    let presentation =
        resolve_choice_presentation(runner.current_choice_presentation_key(dialogue));
    let Some(navigation_policy) = navigation_policy_for_presentation(presentation) else {
        return;
    };

    let Ok(choices) = runner.current_choices(dialogue) else {
        return;
    };
    if choices.is_empty() {
        return;
    }

    let selected_index = runner.clamped_selected_choice_index(choices.len());
    let fallback_index = runner.preferred_choice_index(choices.len()).unwrap_or(0);
    let Some(navigation_result) = apply_navigation_policy(
        navigation_policy,
        &keyboard,
        selected_index,
        fallback_index,
        &choices_container_query,
        &choice_entity_query,
        auto_directional_navigator.as_mut(),
        choices.len(),
    ) else {
        return;
    };

    if let Some(choice_index) = navigation_result.navigated_to {
        select_events.write(SelectDialogueChoice {
            entity,
            choice_index,
        });
    }

    if !confirm_pressed {
        return;
    }

    if runner.state.can_advance() {
        advance_events.write(AdvanceDialogue { entity });
        return;
    }

    select_events.write(SelectDialogueChoice {
        entity,
        choice_index: navigation_result
            .confirm_choice_index
            .min(choices.len().saturating_sub(1)),
    });
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NavigationPolicy {
    LinearVertical,
    Spatial,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NavigationResult {
    navigated_to: Option<usize>,
    confirm_choice_index: usize,
}

fn navigation_policy_for_presentation(mode: ChoicePresentationMode) -> Option<NavigationPolicy> {
    match mode {
        ChoicePresentationMode::StandardList => Some(NavigationPolicy::LinearVertical),
        ChoicePresentationMode::InlineBadges => Some(NavigationPolicy::Spatial),
        ChoicePresentationMode::External => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LinearDirection {
    Up,
    Down,
}

fn apply_navigation_policy(
    navigation_policy: NavigationPolicy,
    keyboard: &ButtonInput<KeyCode>,
    selected_index: Option<usize>,
    fallback_index: usize,
    choices_container_query: &Query<(Entity, &Node, &Children), With<ChoicesContainer>>,
    choice_entity_query: &Query<(Entity, &ChoiceText), With<ChoiceText>>,
    auto_directional_navigator: Option<&mut AutoDirectionalNavigator>,
    choice_count: usize,
) -> Option<NavigationResult> {
    match navigation_policy {
        NavigationPolicy::LinearVertical => {
            let navigated_to =
                linear_navigation_direction(keyboard).map(|direction| match direction {
                    LinearDirection::Up => previous_wrapped_index(selected_index, choice_count),
                    LinearDirection::Down => next_wrapped_index(selected_index, choice_count),
                });

            Some(NavigationResult {
                navigated_to,
                confirm_choice_index: selected_index.unwrap_or(fallback_index),
            })
        }
        NavigationPolicy::Spatial => {
            let Some(auto_directional_navigator) = auto_directional_navigator else {
                return Some(NavigationResult {
                    navigated_to: None,
                    confirm_choice_index: selected_index.unwrap_or(fallback_index),
                });
            };

            let choice_entities =
                active_choice_entities(choices_container_query, choice_entity_query);
            if choice_entities.is_empty() {
                return None;
            }

            let directional_input = spatial_navigation_direction(keyboard);
            let mut focused_index = focused_choice_index(
                auto_directional_navigator
                    .manual_directional_navigation
                    .focus
                    .get(),
                &choice_entities,
            );

            // Keep Bevy's directional focus aligned with the currently selected choice.
            // This ensures directional moves after number-key selection start from the
            // visually selected entry instead of a stale previously focused entry.
            if let Some(selected) = selected_index
                && focused_index != Some(selected)
            {
                focused_index = set_spatial_focus_to_index(
                    auto_directional_navigator,
                    &choice_entities,
                    selected,
                )
                .or(focused_index);
            }

            let had_focused_choice = focused_index.is_some();

            // Only seed focus when navigation is attempted, so idle frames do not silently
            // establish an internal focus state that disagrees with the visual selection state.
            if directional_input.is_some() && focused_index.is_none() {
                focused_index = ensure_spatial_focus(
                    auto_directional_navigator,
                    &choice_entities,
                    fallback_index,
                );
            }

            // When entering spatial mode with no explicit selection yet, the first directional
            // input should establish focus only. Actual movement starts on the next press.
            if directional_input.is_some() && !had_focused_choice && selected_index.is_none() {
                return Some(NavigationResult {
                    navigated_to: focused_index,
                    confirm_choice_index: focused_index.unwrap_or(fallback_index),
                });
            }

            let navigated_to = directional_input.and_then(|direction| {
                if auto_directional_navigator.navigate(direction).is_ok() {
                    focused_index = focused_choice_index(
                        auto_directional_navigator
                            .manual_directional_navigation
                            .focus
                            .get(),
                        &choice_entities,
                    );
                }
                focused_index
            });

            Some(NavigationResult {
                navigated_to,
                confirm_choice_index: focused_index.unwrap_or(fallback_index),
            })
        }
    }
}

fn linear_navigation_direction(keyboard: &ButtonInput<KeyCode>) -> Option<LinearDirection> {
    if keyboard.just_pressed(KeyCode::ArrowUp) {
        Some(LinearDirection::Up)
    } else if keyboard.just_pressed(KeyCode::ArrowDown) {
        Some(LinearDirection::Down)
    } else {
        None
    }
}

fn active_choice_entities(
    choices_container_query: &Query<(Entity, &Node, &Children), With<ChoicesContainer>>,
    choice_entity_query: &Query<(Entity, &ChoiceText), With<ChoiceText>>,
) -> Vec<(usize, Entity)> {
    let Some(active_children) = choices_container_query
        .iter()
        .filter(|(_, node, _)| node.display != Display::None)
        .min_by_key(|(entity, _, _)| entity.to_bits())
        .map(|(_, _, children)| children)
    else {
        return Vec::new();
    };

    let mut entities: Vec<(usize, Entity)> = active_children
        .iter()
        .filter_map(|child| {
            choice_entity_query
                .get(child)
                .ok()
                .map(|(entity, choice_text)| (choice_text.0, entity))
        })
        .collect();
    entities.sort_by_key(|(index, _)| *index);
    entities
}

fn focused_choice_index(
    focused_entity: Option<Entity>,
    choice_entities: &[(usize, Entity)],
) -> Option<usize> {
    let focused_entity = focused_entity?;
    choice_entities
        .iter()
        .find_map(|(index, entity)| (*entity == focused_entity).then_some(*index))
}

fn ensure_spatial_focus(
    auto_directional_navigator: &mut AutoDirectionalNavigator,
    choice_entities: &[(usize, Entity)],
    fallback_index: usize,
) -> Option<usize> {
    let focused_index = focused_choice_index(
        auto_directional_navigator
            .manual_directional_navigation
            .focus
            .get(),
        choice_entities,
    );
    if focused_index.is_some() {
        return focused_index;
    }

    let seed_entity = choice_entities
        .iter()
        .find_map(|(index, entity)| (*index == fallback_index).then_some(*entity))
        .unwrap_or(choice_entities[0].1);
    auto_directional_navigator
        .manual_directional_navigation
        .focus
        .set(seed_entity);

    focused_choice_index(
        auto_directional_navigator
            .manual_directional_navigation
            .focus
            .get(),
        choice_entities,
    )
}

fn set_spatial_focus_to_index(
    auto_directional_navigator: &mut AutoDirectionalNavigator,
    choice_entities: &[(usize, Entity)],
    target_index: usize,
) -> Option<usize> {
    let target_entity = choice_entities
        .iter()
        .find_map(|(index, entity)| (*index == target_index).then_some(*entity))?;
    auto_directional_navigator
        .manual_directional_navigation
        .focus
        .set(target_entity);
    Some(target_index)
}

fn spatial_navigation_direction(keyboard: &ButtonInput<KeyCode>) -> Option<CompassOctant> {
    if keyboard.just_pressed(KeyCode::ArrowLeft) {
        Some(CompassOctant::West)
    } else if keyboard.just_pressed(KeyCode::ArrowRight) {
        Some(CompassOctant::East)
    } else if keyboard.just_pressed(KeyCode::ArrowUp) {
        Some(CompassOctant::North)
    } else if keyboard.just_pressed(KeyCode::ArrowDown) {
        Some(CompassOctant::South)
    } else {
        None
    }
}

fn previous_wrapped_index(selected_index: Option<usize>, len: usize) -> usize {
    if len == 0 {
        return 0;
    }

    match selected_index {
        Some(index) => {
            if index == 0 {
                len - 1
            } else {
                index - 1
            }
        }
        None => len - 1,
    }
}

fn next_wrapped_index(selected_index: Option<usize>, len: usize) -> usize {
    if len == 0 {
        return 0;
    }

    match selected_index {
        Some(index) => (index + 1) % len,
        None => 0,
    }
}
