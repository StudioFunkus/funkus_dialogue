//! A simple example demonstrating basic dialogue functionality with debugging tools.

use bevy::prelude::*;
use funkus_dialogue_core::*;
use funkus_dialogue_ui::*;

fn main() {
    let mut app = App::new();

    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Dialogue System Example (with Debug UI)".to_string(),
                resolution: (1280.0, 720.0).into(),
                ..default()
            }),
            ..default()
        }),
        DialoguePlugin,
        DialogueUIPlugin,
    ))
    .add_systems(Startup, setup)
    .add_systems(Update, keyboard_input);

    app.run();
}
/// Sets up the example scene.
fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Create a camera
    commands.spawn(Camera2d);

    // Create an entity to run the dialogue
    commands.spawn((Name::new("Guide Conversation"), DialogueRunner::default()));

    // Load a dialogue asset
    let dialogue_handle = asset_server.load("dialogues/example.dialogue.json");

    // Print a message about controls
    info!("Press SPACE to advance dialogue or confirm choices, 1-9 to select choices, ESC to exit");

    // Add some UI elements - title
    commands.spawn((
        Text::new("Dialogue System Example"),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));

    // Add controls info
    commands.spawn((
        Text::new("Controls: SPACE to advance text/confirm choice, 1-9 for choices, ESC to exit"),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));

    // Loading text
    commands.spawn((
        Text::new("Loading dialogue..."),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(50.0),
            left: Val::Px(50.0),
            ..default()
        },
        LoadingText,
    ));

    // Spawn the dialogue UI
    spawn_dialogue_ui(&mut commands);

    // Schedule the dialogue to start once loaded
    commands.insert_resource(DialogueToStart(dialogue_handle));
}

// Resource to track the dialogue to start
#[derive(Resource)]
struct DialogueToStart(Handle<DialogueAsset>);

/// System to handle keyboard input.
fn keyboard_input(
    mut commands: Commands,
    dialogue_assets: Res<Assets<DialogueAsset>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    dialogue_to_start: Option<Res<DialogueToStart>>,
    mut dialogue_query: Query<(Entity, &mut DialogueRunner)>,
    mut advance_events: EventWriter<AdvanceDialogue>,
    mut select_events: EventWriter<SelectDialogueChoice>,
    mut start_events: EventWriter<StartDialogue>,
    mut stop_events: EventWriter<StopDialogue>,
    text_query: Query<Entity, With<LoadingText>>,
) {
    // Check if we need to start the dialogue
    if let Some(dialogue_to_start) = dialogue_to_start.as_ref() {
        if dialogue_assets.contains(&dialogue_to_start.0) {
            // Get the dialogue entity
            if let Some((entity, _)) = dialogue_query.iter().next() {
                // Start the dialogue
                start_events.write(StartDialogue {
                    entity,
                    dialogue_handle: dialogue_to_start.0.clone(),
                });

                // Remove the resource
                commands.remove_resource::<DialogueToStart>();

                // Update the loading text
                for loading_entity in text_query.iter() {
                    // Remove the loading text
                    commands.entity(loading_entity).despawn();
                }
            }
        }
    }

    // Handle input for all active dialogues
    for (entity, runner) in dialogue_query.iter_mut() {
        // Only process input if the dialogue is active
        if runner.state == DialogueState::Inactive
            || runner.state == DialogueState::Error("".to_string())
        {
            continue;
        }

        // Space key should now advance after a choice is selected or for text nodes
        if keyboard_input.just_pressed(KeyCode::Space) {
            // Check the current state to determine if we should advance
            match runner.state {
                DialogueState::ShowingText => {
                    // Normal text advancement
                    advance_events.write(AdvanceDialogue { entity });
                }
                DialogueState::ChoiceSelected(_) => {
                    // Advance after a choice is selected
                    advance_events.write(AdvanceDialogue { entity });
                }
                _ => {
                    // No action for other states
                }
            }
        }

        // Escape to stop
        if keyboard_input.just_pressed(KeyCode::Escape) {
            stop_events.write(StopDialogue { entity });
        }

        // Number keys for choices - allow changing choice even after initial selection
        if runner.state == DialogueState::WaitingForChoice
            || matches!(runner.state, DialogueState::ChoiceSelected(_))
        {
            for i in 0..9 {
                let key = match i {
                    0 => KeyCode::Digit1,
                    1 => KeyCode::Digit2,
                    2 => KeyCode::Digit3,
                    3 => KeyCode::Digit4,
                    4 => KeyCode::Digit5,
                    5 => KeyCode::Digit6,
                    6 => KeyCode::Digit7,
                    7 => KeyCode::Digit8,
                    8 => KeyCode::Digit9,
                    _ => unreachable!(),
                };

                if keyboard_input.just_pressed(key) {
                    // Only send the selection event - don't advance immediately
                    select_events.write(SelectDialogueChoice {
                        entity,
                        choice_index: i,
                    });
                    break;
                }
            }
        }
    }
}
