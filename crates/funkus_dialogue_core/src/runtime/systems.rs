//! # Systems for dialogue processing.
//!
//! This module provides the Bevy systems that handle dialogue runtime processing,
//! including system setup, event handling, and dialogue state updates.

use bevy::prelude::*;

use crate::asset::DialogueAsset;
use crate::runtime::DialogueRunner;
use crate::runtime::DialogueState;

/// System that updates all dialogue runners.
///
/// This system is responsible for:
/// - Ticking auto-advance timers
/// - Auto-advancing text nodes when the timer completes
/// - Handling other state updates
///
/// Note: The system automatically skips runners with inactive state or
/// runners whose dialogue assets haven't been loaded yet. It will silently
/// continue processing other runners without errors.
///
/// # System Parameters
///
/// * `time` - The Bevy time resource for delta time
/// * `dialogue_assets` - Assets resource containing loaded dialogue assets
/// * `runner_query` - Query for DialogueRunner components
///
/// # Example
///
/// ```rust
/// use bevy::prelude::*;
/// use funkus_dialogue::runtime::update_dialogue_runners;
///
/// fn setup_app(app: &mut App) {
///     app.add_systems(Update, update_dialogue_runners);
/// }
/// ```
pub fn update_dialogue_runners(
    time: Res<Time>,
    dialogue_assets: Res<Assets<DialogueAsset>>,
    mut runner_query: Query<&mut DialogueRunner>,
) {
    for mut runner in runner_query.iter_mut() {
        // Skip inactive runners
        if runner.state == DialogueState::Inactive {
            continue;
        }

        // Get the dialogue asset
        let Some(dialogue) = dialogue_assets.get(&runner.dialogue_handle) else {
            // Asset not loaded yet
            continue;
        };

        // Auto-advance text nodes if enabled
        if runner.state == DialogueState::ShowingText && runner.auto_advance {
            runner.auto_advance_timer.tick(time.delta());

            if runner.auto_advance_timer.finished() {
                if let Err(err) = runner.advance(dialogue) {
                    error!("Error advancing dialogue: {}", err);
                    runner.state = DialogueState::Error(err.to_string());
                }
            }
        }
    }
}

/// System set for dialogue processing.
///
/// This system set groups all dialogue-related systems to allow for
/// proper scheduling and dependencies.
///
/// # Example
///
/// ```rust
/// use bevy::prelude::*;
/// use funkus_dialogue::runtime::DialogueSystemSet;
///
/// fn setup_app(app: &mut App) {
///     app.configure_sets(Update, DialogueSystemSet);
///     
///     // Add systems to the dialogue set
///     app.add_systems(Update, my_dialogue_system.in_set(DialogueSystemSet));
/// }
/// ```
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct DialogueSystemSet;

/// System for handling dialogue events.
///
/// This system processes all dialogue-related events, including:
/// - Starting dialogues
/// - Stopping dialogues
/// - Advancing to the next node
/// - Selecting choices
///
/// It also sends appropriate events to notify other systems about
/// dialogue state changes.
///
/// # System Parameters
///
/// * `commands` - Bevy commands for entity management
/// * `dialogue_assets` - Assets resource containing loaded dialogue assets
/// * `start_events` - EventReader for StartDialogue events
/// * `stop_events` - EventReader for StopDialogue events
/// * `advance_events` - EventReader for AdvanceDialogue events
/// * `select_events` - EventReader for SelectDialogueChoice events
/// * `node_activated_events` - EventWriter for DialogueNodeActivated events
/// * `dialogue_started_events` - EventWriter for DialogueStarted events
/// * `dialogue_ended_events` - EventWriter for DialogueEnded events
/// * `dialogue_choice_events` - EventWriter for DialogueChoiceMade events
/// * `runner_query` - Query for DialogueRunner components
pub fn handle_dialogue_events(
    mut commands: Commands,
    dialogue_assets: Res<Assets<DialogueAsset>>,
    mut start_events: EventReader<crate::events::StartDialogue>,
    mut stop_events: EventReader<crate::events::StopDialogue>,
    mut advance_events: EventReader<crate::events::AdvanceDialogue>,
    mut select_events: EventReader<crate::events::SelectDialogueChoice>,
    mut node_activated_events: EventWriter<crate::events::DialogueNodeActivated>,
    mut dialogue_started_events: EventWriter<crate::events::DialogueStarted>,
    mut dialogue_ended_events: EventWriter<crate::events::DialogueEnded>,
    mut dialogue_choice_events: EventWriter<crate::events::DialogueChoiceMade>,
    mut runner_query: Query<&mut DialogueRunner>,
) {
    // Handle start dialogue events
    for ev in start_events.read() {
        if let Ok(mut runner) = runner_query.get_mut(ev.entity) {
            // Set the dialogue handle
            runner.dialogue_handle = ev.dialogue_handle.clone();

            // Get the dialogue asset
            if let Some(dialogue) = dialogue_assets.get(&ev.dialogue_handle) {
                // Start the dialogue
                runner.start(dialogue);

                // Send node activated event for the start node
                if let Some(node_id) = runner.current_node_id {
                    node_activated_events.write(crate::events::DialogueNodeActivated {
                        entity: ev.entity,
                        node_id,
                    });

                    // Send dialogue started event
                    dialogue_started_events.write(crate::events::DialogueStarted {
                        entity: ev.entity,
                        start_node_id: node_id,
                    });
                }
            }
        } else {
            // Create a new dialogue runner component
            commands
                .entity(ev.entity)
                .insert(DialogueRunner::new(ev.dialogue_handle.clone()));
        }
    }

    // Handle stop dialogue events
    for ev in stop_events.read() {
        if let Ok(mut runner) = runner_query.get_mut(ev.entity) {
            // Send dialogue ended event
            dialogue_ended_events.write(crate::events::DialogueEnded {
                entity: ev.entity,
                normal_exit: false,
            });

            // Stop the dialogue
            runner.stop();
        }
    }

    // Handle advance dialogue events
    for ev in advance_events.read() {
        if let Ok(mut runner) = runner_query.get_mut(ev.entity) {
            // Get the dialogue asset
            if let Some(dialogue) = dialogue_assets.get(&runner.dialogue_handle) {
                let old_node_id = runner.current_node_id;

                // Advance the dialogue
                match runner.advance(dialogue) {
                    Ok(()) => {
                        if runner.state == DialogueState::Finished {
                            // Send dialogue ended event
                            dialogue_ended_events.write(crate::events::DialogueEnded {
                                entity: ev.entity,
                                normal_exit: true,
                            });
                        } else if runner.current_node_id != old_node_id {
                            // Send node activated event
                            if let Some(node_id) = runner.current_node_id {
                                node_activated_events.write(crate::events::DialogueNodeActivated {
                                    entity: ev.entity,
                                    node_id,
                                });
                            }
                        }
                    }
                    Err(err) => {
                        error!("Error advancing dialogue: {}", err);
                        runner.state = DialogueState::Error(err.to_string());
                    }
                }
            }
        }
    }

    // Handle select choice events
    for ev in select_events.read() {
        if let Ok(mut runner) = runner_query.get_mut(ev.entity) {
            // Allow choice selection while in either WaitingForChoice or ChoiceSelected state
            if runner.state == DialogueState::WaitingForChoice
                || matches!(runner.state, DialogueState::ChoiceSelected(_))
            {
                // Get the current node id
                let Some(node_id) = runner.current_node_id else {
                    continue;
                };

                // Select the choice - this now also updates the state to ChoiceSelected
                if let Err(err) = runner.select_choice(ev.choice_index) {
                    error!("Error selecting choice: {}", err);
                }

                // Send choice made event
                dialogue_choice_events.write(crate::events::DialogueChoiceMade {
                    entity: ev.entity,
                    node_id,
                    choice_index: ev.choice_index,
                });
            }
        }
    }
}

/// Set up the dialogue systems.
///
/// This function registers all dialogue-related systems with the Bevy app,
/// configuring them with the appropriate system set for scheduling.
///
/// # Parameters
///
/// * `app` - The Bevy App to configure
///
/// # Example
///
/// ```rust
/// use bevy::prelude::*;
/// use funkus_dialogue::runtime::setup_dialogue_systems;
///
/// fn main() {
///     let mut app = App::new();
///     setup_dialogue_systems(&mut app);
///     // ... add other app configuration
///     app.run();
/// }
/// ```
pub fn setup_dialogue_systems(app: &mut App) {
    app.configure_sets(Update, DialogueSystemSet).add_systems(
        Update,
        (update_dialogue_runners, handle_dialogue_events).in_set(DialogueSystemSet),
    );
}
