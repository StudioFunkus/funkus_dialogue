use bevy::prelude::*;

/// Component for dialogue display container
#[derive(Component)]
pub struct DialogueDisplay;

/// Component for speaker text
#[derive(Component)]
pub struct SpeakerText;

/// Component for dialogue text
#[derive(Component)]
pub struct DialogueText;

/// Component for choices container
#[derive(Component)]
pub struct ChoicesContainer;

/// Component for individual choice options
#[derive(Component)]
pub struct ChoiceText(pub usize);

/// Component for loading text indicator
#[derive(Component)]
pub struct LoadingText;
