use bevy::ecs::schedule::common_conditions::on_message;
use bevy::prelude::*;
use funkus_dialogue_core::{
    DialogueChoicePresentation, DialogueChoicePresentationAppExt, DialogueEnded, DialogueStarted,
    DialogueSystemSet,
};

use crate::systems;

/// Built-in alternate key used by this crate's inline badge choice renderer.
pub const INLINE_BADGES_PRESENTATION_KEY: &str = "inline_badges";

/// Metadata registration for the built-in inline badge renderer.
pub struct InlineBadgesChoicePresentation;

impl DialogueChoicePresentation for InlineBadgesChoicePresentation {
    fn key() -> &'static str {
        INLINE_BADGES_PRESENTATION_KEY
    }

    fn label() -> &'static str {
        "Inline Badges"
    }

    fn description() -> Option<&'static str> {
        Some("Renders options as inline badge-like choices")
    }
}

/// Plugin for dialogue UI functionality.
pub struct DialogueUIPlugin;

impl Plugin for DialogueUIPlugin {
    fn build(&self, app: &mut App) {
        app.register_choice_presentation::<InlineBadgesChoicePresentation>()
            .init_resource::<systems::DialogueUiLifecycleState>()
            .add_systems(
                Update,
                (
                    systems::unmount_dialogue_ui_on_end.run_if(on_message::<DialogueEnded>),
                    systems::mount_dialogue_ui_on_start.run_if(on_message::<DialogueStarted>),
                    systems::display_dialogue,
                    systems::default_choice_input,
                )
                    .chain()
                    .after(DialogueSystemSet),
            );
    }
}
