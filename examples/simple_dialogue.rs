//! A simple example demonstrating basic dialogue functionality with debugging tools.

use bevy::prelude::*;
use funkus_dialogue::graph::DialogueNode;
use funkus_dialogue::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Dialogue System Example (with Debug UI)".to_string(),
                    resolution: (1280.0, 720.0).into(),
                    ..default()
                }),
                ..default()
            }),
            DialoguePlugin,
            #[cfg(feature = "debug_ui")]
            DialogueDebugPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (keyboard_input, display_dialogue))
        .run();
}

/// Sets up the example scene.
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    // Create a camera
    commands.spawn(Camera2d);
    
    // Create an entity to run the dialogue
    commands.spawn((
        Name::new("Guide Conversation"),
        DialogueRunner::default(),
    ));
    
    // Load a dialogue asset
    let dialogue_handle = asset_server.load("dialogues/example.dialogue.json");
    
    // Print a message about controls
    info!("Press SPACE to advance dialogue or confirm choices, 1-9 to select choices, ESC to exit");
    info!("Press F1 to toggle debug UI");
    
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
        Text::new("Controls: SPACE to advance text/confirm choice, 1-9 for choices, ESC to exit, F1 for debug"),
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
    
    // Schedule the dialogue to start once loaded
    commands.insert_resource(DialogueToStart(dialogue_handle));
}

// Resource to track the dialogue to start
#[derive(Resource)]
struct DialogueToStart(Handle<DialogueAsset>);

// Component for loading text
#[derive(Component)]
struct LoadingText;

// Component for dialogue display
#[derive(Component)]
struct DialogueDisplay;

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
                start_events.send(StartDialogue {
                    entity,
                    dialogue_handle: dialogue_to_start.0.clone(),
                });
                
                // Remove the resource
                commands.remove_resource::<DialogueToStart>();
                
                // Update the loading text
                for loading_entity in text_query.iter() {
                    // Remove the loading text
                    commands.entity(loading_entity).despawn();
                    
                    // Spawn dialogue display UI
                    commands.spawn((
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
                        BackgroundColor(Color::rgba(0.1, 0.1, 0.1, 0.8)),
                        DialogueDisplay,
                    )).with_children(|parent| {
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
                    });
                }
            }
        }
    }
    
    // Handle input for all active dialogues
    for (entity, runner) in dialogue_query.iter_mut() {
        // Only process input if the dialogue is active
        if runner.state == DialogueState::Inactive || runner.state == DialogueState::Error("".to_string()) {
            continue;
        }
        
        // Space key should now advance after a choice is selected or for text nodes
        if keyboard_input.just_pressed(KeyCode::Space) {
            // Check the current state to determine if we should advance
            match runner.state {
                DialogueState::ShowingText => {
                    // Normal text advancement
                    advance_events.send(AdvanceDialogue {
                        entity,
                    });
                },
                DialogueState::ChoiceSelected(_) => {
                    // Advance after a choice is selected
                    advance_events.send(AdvanceDialogue {
                        entity,
                    });
                },
                _ => {
                    // No action for other states
                }
            }
        }
        
        // Escape to stop
        if keyboard_input.just_pressed(KeyCode::Escape) {
            stop_events.send(StopDialogue {
                entity,
            });
        }
        
        // Number keys for choices - allow changing choice even after initial selection
        if runner.state == DialogueState::WaitingForChoice || matches!(runner.state, DialogueState::ChoiceSelected(_)) {
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
                    select_events.send(SelectDialogueChoice {
                        entity,
                        choice_index: i,
                    });
                    break;
                }
            }
        }
    }
}

// Components for dialogue UI
#[derive(Component)]
struct SpeakerText;

#[derive(Component)]
struct DialogueText;

#[derive(Component)]
struct ChoicesContainer;

#[derive(Component)]
struct ChoiceText(usize);

/// System to display dialogue content.
fn display_dialogue(
    mut commands: Commands,
    dialogue_assets: Res<Assets<DialogueAsset>>,
    dialogue_query: Query<(&DialogueRunner, &Name)>,
    mut speaker_query: Query<&mut Text, With<SpeakerText>>,
    mut dialogue_query_text: Query<&mut Text, (With<DialogueText>, Without<SpeakerText>, Without<ChoiceText>)>,
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
                        NodeType::Text(text_node) => {
                            // Update speaker
                            for mut speaker_text in speaker_query.iter_mut() {
                                if let Some(speaker) = &text_node.speaker {
                                    *speaker_text = Text::new(speaker.clone());
                                } else {
                                    *speaker_text = Text::new("");
                                }
                            }
                            
                            // Update dialogue text
                            for mut dialogue_text in dialogue_query_text.iter_mut() {
                                *dialogue_text = Text::new(text_node.text.clone());
                            }
                            
                            // Clear choices
                            for choices_entity in choices_query.iter() {
                                commands.entity(choices_entity).despawn_descendants();
                            }
                        },
                        NodeType::Choice(choice_node) => {
                            // Update speaker
                            for mut speaker_text in speaker_query.iter_mut() {
                                if let Some(speaker) = &choice_node.speaker {
                                    *speaker_text = Text::new(speaker.clone());
                                } else {
                                    *speaker_text = Text::new("");
                                }
                            }
                            
                            // Update dialogue text (prompt)
                            for mut dialogue_text in dialogue_query_text.iter_mut() {
                                if let Some(prompt) = &choice_node.prompt {
                                    *dialogue_text = Text::new(prompt.clone());
                                } else {
                                    *dialogue_text = Text::new("Choose an option:");
                                }
                            }
                            
                            // Update choices
                            let connections = choice_node.connections();
                            
                            // Handle the ChoiceSelected state
                            let selected_index = match runner.state {
                                DialogueState::ChoiceSelected(index) => Some(index),
                                _ => None,
                            };
                            
                            for choices_entity in choices_query.iter() {
                                commands.entity(choices_entity).despawn_descendants();
                                
                                // Add choice buttons
                                for (i, conn) in connections.iter().enumerate() {
                                    let choice_text = conn.label.clone().unwrap_or_else(|| format!("Choice {}", i+1));
                                    
                                    let display_text = if Some(i) == selected_index {
                                        // Highlight selected choice
                                        format!("▶ {}. {}", i+1, choice_text)
                                    } else {
                                        format!("{}. {}", i+1, choice_text)
                                    };
                                    
                                    commands.entity(choices_entity).with_children(|parent| {
                                        parent.spawn((
                                            Text::new(display_text),
                                            TextFont {
                                                font_size: 16.0,
                                                ..default()
                                            },
                                            TextColor(if Some(i) == selected_index {
                                                Color::rgb(1.0, 1.0, 0.5) // Highlight selected choice
                                            } else {
                                                Color::rgb(0.8, 0.8, 1.0)
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
                        },
                    }
                }
            }
        }
    }
}