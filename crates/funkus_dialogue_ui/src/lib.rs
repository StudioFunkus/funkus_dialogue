//! # Funkus Dialogue UI
//!
//! UI components for displaying dialogues created with the funkus_dialogue system.

use bevy::prelude::*;

// Components specific to dialogue UI
mod components;
mod systems;

pub use components::*;

/// Plugin for dialogue UI functionality
pub struct DialogueUIPlugin;

impl Plugin for DialogueUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, systems::display_dialogue);
    }
}

/// Bundle for adding dialogue UI components to an entity
#[derive(Bundle)]
pub struct DialogueUIBundle {
    pub display: DialogueDisplay,
}

/// Function to spawn a dialogue UI
pub fn spawn_dialogue_ui(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(60.0),
                left: Val::Px(100.0),
                right: Val::Px(100.0),
                height: Val::Px(200.0),
                padding: UiRect::all(Val::Px(10.0)),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.8)),
            DialogueDisplay,
        ))
        .with_children(|parent| {
            // Speaker name
            parent.spawn((
                Text::new(""),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                SpeakerText,
            ));

            // Dialogue text
            parent.spawn((
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

            // Choices container
            parent.spawn((
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    margin: UiRect::top(Val::Px(20.0)),
                    ..default()
                },
                ChoicesContainer,
            ));
        })
        .id()
}
