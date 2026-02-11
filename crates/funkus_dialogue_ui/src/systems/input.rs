use bevy::prelude::*;
use funkus_dialogue_core::{
    AdvanceDialogue, DialogueAsset, DialogueRunner, DialogueState, SelectDialogueChoice,
};

use crate::presentation::resolve_choice_presentation;

/// Keyboard navigation for built-in choice presentations.
///
/// This system intentionally ignores unknown external presentation keys so game-specific UIs
/// can provide their own input behavior.
pub fn default_choice_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    dialogue_assets: Res<Assets<DialogueAsset>>,
    dialogue_query: Query<(Entity, &DialogueRunner)>,
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

    if !(runner.state == DialogueState::WaitingForChoice
        || matches!(runner.state, DialogueState::ChoiceSelected(_)))
    {
        return;
    }

    let Some(dialogue) = dialogue_assets.get(&runner.dialogue_handle) else {
        return;
    };

    let presentation =
        resolve_choice_presentation(runner.current_choice_presentation_key(dialogue));
    if !presentation.supports_builtin_input() {
        return;
    }

    let Ok(choices) = runner.current_choices(dialogue) else {
        return;
    };
    if choices.is_empty() {
        return;
    }

    let current_index = match runner.state {
        DialogueState::ChoiceSelected(index) => index.min(choices.len().saturating_sub(1)),
        _ => 0,
    };

    if keyboard.just_pressed(KeyCode::ArrowUp) {
        let next = if current_index == 0 {
            choices.len() - 1
        } else {
            current_index - 1
        };
        select_events.write(SelectDialogueChoice {
            entity,
            choice_index: next,
        });
    }

    if keyboard.just_pressed(KeyCode::ArrowDown) {
        let next = (current_index + 1) % choices.len();
        select_events.write(SelectDialogueChoice {
            entity,
            choice_index: next,
        });
    }

    let confirm_pressed = keyboard.just_pressed(KeyCode::Enter)
        || keyboard.just_pressed(KeyCode::NumpadEnter)
        || keyboard.just_pressed(KeyCode::Space);
    if !confirm_pressed {
        return;
    }

    if let DialogueState::ChoiceSelected(_) = runner.state {
        advance_events.write(AdvanceDialogue { entity });
        return;
    }

    select_events.write(SelectDialogueChoice {
        entity,
        choice_index: current_index,
    });
}
