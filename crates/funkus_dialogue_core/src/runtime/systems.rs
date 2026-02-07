//! # Systems for dialogue processing.
//!
//! This module provides the Bevy systems that handle dialogue runtime processing,
//! including system setup, event handling, and dialogue state updates.

use bevy::ecs::message::{MessageCursor, Messages};
use bevy::prelude::*;
use tracing::{error, warn};

use crate::asset::DialogueAsset;
use crate::registry::{DialogueMessageRegistry, DialogueRegistry};
use crate::runtime::DialogueRunner;
use crate::runtime::DialogueState;
use crate::{AdvanceDialogue, DialogueNodeActivated};

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
/// ```rust,ignore
/// use bevy::prelude::*;
/// use funkus_dialogue_core::runtime::update_dialogue_runners;
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

            if runner.auto_advance_timer.is_finished()
                && let Err(err) = runner.advance(dialogue)
            {
                error!("Error advancing dialogue: {}", err);
                runner.state = DialogueState::Error(err.to_string());
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
/// ```rust,ignore
/// use bevy::prelude::*;
/// use funkus_dialogue_core::runtime::DialogueSystemSet;
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

#[derive(Resource, Default)]
struct DialogueActionCursor(MessageCursor<DialogueNodeActivated>);

/// Handle dialogue start requests.
pub fn handle_start_dialogue_events(
    dialogue_assets: Res<Assets<DialogueAsset>>,
    mut start_events: MessageReader<crate::events::StartDialogue>,
    mut node_activated_events: MessageWriter<crate::events::DialogueNodeActivated>,
    mut dialogue_started_events: MessageWriter<crate::events::DialogueStarted>,
    mut runner_query: Query<&mut DialogueRunner>,
) {
    for ev in start_events.read() {
        let Ok(mut runner) = runner_query.get_mut(ev.entity) else {
            warn!(
                "Ignoring StartDialogue for {:?}: entity has no DialogueRunner",
                ev.entity
            );
            continue;
        };
        let Some(dialogue) = dialogue_assets.get(&ev.dialogue_handle) else {
            warn!(
                "Ignoring StartDialogue for {:?}: dialogue asset is not loaded yet",
                ev.entity
            );
            continue;
        };

        match runner.start(dialogue) {
            Ok(()) => {
                runner.dialogue_handle = ev.dialogue_handle.clone();
                if let Some(node_id) = runner.current_node_id {
                    node_activated_events.write(crate::events::DialogueNodeActivated {
                        entity: ev.entity,
                        node_id,
                    });

                    dialogue_started_events.write(crate::events::DialogueStarted {
                        entity: ev.entity,
                        start_node_id: node_id,
                    });
                }
            }
            Err(err) => {
                runner.state = DialogueState::Error(err.to_string());
                runner.current_node_id = None;
                error!("Failed to start dialogue for {:?}: {}", ev.entity, err);
            }
        }
    }
}

/// Handle dialogue stop requests.
pub fn handle_stop_dialogue_events(
    mut stop_events: MessageReader<crate::events::StopDialogue>,
    mut dialogue_ended_events: MessageWriter<crate::events::DialogueEnded>,
    mut runner_query: Query<&mut DialogueRunner>,
) {
    for ev in stop_events.read() {
        if let Ok(mut runner) = runner_query.get_mut(ev.entity) {
            dialogue_ended_events.write(crate::events::DialogueEnded {
                entity: ev.entity,
                normal_exit: false,
            });

            runner.stop();
        }
    }
}

/// Handle dialogue advance requests.
pub fn handle_advance_dialogue_events(
    dialogue_assets: Res<Assets<DialogueAsset>>,
    mut advance_events: MessageReader<crate::events::AdvanceDialogue>,
    mut node_activated_events: MessageWriter<crate::events::DialogueNodeActivated>,
    mut dialogue_ended_events: MessageWriter<crate::events::DialogueEnded>,
    mut runner_query: Query<&mut DialogueRunner>,
) {
    for ev in advance_events.read() {
        if let Ok(mut runner) = runner_query.get_mut(ev.entity) {
            if let Some(dialogue) = dialogue_assets.get(&runner.dialogue_handle) {
                let old_node_id = runner.current_node_id;

                match runner.advance(dialogue) {
                    Ok(()) => {
                        if runner.state == DialogueState::Finished {
                            dialogue_ended_events.write(crate::events::DialogueEnded {
                                entity: ev.entity,
                                normal_exit: true,
                            });
                        } else if runner.current_node_id != old_node_id {
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
}

/// Handle dialogue choice selection requests.
pub fn handle_select_dialogue_events(
    dialogue_assets: Res<Assets<DialogueAsset>>,
    mut select_events: MessageReader<crate::events::SelectDialogueChoice>,
    mut dialogue_choice_events: MessageWriter<crate::events::DialogueChoiceMade>,
    mut runner_query: Query<&mut DialogueRunner>,
) {
    for ev in select_events.read() {
        if let Ok(mut runner) = runner_query.get_mut(ev.entity) {
            if runner.state == DialogueState::WaitingForChoice
                || matches!(runner.state, DialogueState::ChoiceSelected(_))
            {
                let Some(dialogue) = dialogue_assets.get(&runner.dialogue_handle) else {
                    continue;
                };

                let Some(node_id) = runner.current_node_id else {
                    continue;
                };

                let connections = dialogue.graph.get_connected_nodes(node_id);
                if connections.is_empty() || ev.choice_index >= connections.len() {
                    warn!(
                        "Ignoring invalid choice index {} for entity {:?} (available choices: {})",
                        ev.choice_index,
                        ev.entity,
                        connections.len()
                    );
                    continue;
                }

                if let Err(err) = runner.select_choice(ev.choice_index) {
                    error!("Error selecting choice: {}", err);
                    continue;
                }

                dialogue_choice_events.write(crate::events::DialogueChoiceMade {
                    entity: ev.entity,
                    node_id,
                    choice_index: ev.choice_index,
                });
            }
        }
    }
}

/// Apply non-visual dialogue nodes (effects/messages) and auto-advance.
fn apply_dialogue_actions(world: &mut World) {
    let events: Vec<DialogueNodeActivated> =
        world.resource_scope(|world, mut cursor: Mut<DialogueActionCursor>| {
            let messages = world.resource::<Messages<DialogueNodeActivated>>();
            cursor.0.read(messages).cloned().collect()
        });

    if events.is_empty() {
        return;
    }

    enum NodeAction {
        Effect(crate::registry::DialogueEffect),
        Message(crate::registry::DialogueMessageCall),
    }

    for event in events {
        let dialogue_handle = match world.get::<DialogueRunner>(event.entity) {
            Some(runner) => runner.dialogue_handle.clone(),
            None => continue,
        };
        let action = {
            let dialogue_assets = world.resource::<Assets<DialogueAsset>>();
            let Some(dialogue) = dialogue_assets.get(&dialogue_handle) else {
                continue;
            };
            let Some(node) = dialogue.graph.get_node(event.node_id) else {
                continue;
            };
            match node {
                crate::graph::DialogueNode::Effect { effect } => NodeAction::Effect(effect.clone()),
                crate::graph::DialogueNode::Message { message } => {
                    NodeAction::Message(message.clone())
                }
                _ => continue,
            }
        };

        match action {
            NodeAction::Effect(effect) => {
                let field = world
                    .resource::<DialogueRegistry>()
                    .field(&effect.key)
                    .cloned();
                let Some(field) = field else {
                    warn!("Dialogue effect key {} is not registered", effect.key);
                    continue;
                };

                let reflect_from_ptr = {
                    let type_registry = world.resource::<AppTypeRegistry>();
                    let type_registry = type_registry.read();
                    crate::registry::resolve_reflect_from_ptr(&type_registry, &field)
                };

                let Ok(reflect_from_ptr) = reflect_from_ptr else {
                    error!("Missing ReflectFromPtr for {}", effect.key);
                    continue;
                };

                if let Err(err) = crate::registry::apply_effect_with_field_and_reflect(
                    world,
                    &field,
                    &reflect_from_ptr,
                    &effect,
                ) {
                    error!("Failed to apply dialogue effect {}: {}", effect.key, err);
                    continue;
                }
                world
                    .resource_mut::<Messages<AdvanceDialogue>>()
                    .write(AdvanceDialogue {
                        entity: event.entity,
                    });
            }
            NodeAction::Message(message) => {
                let dispatch = world
                    .get_resource::<DialogueMessageRegistry>()
                    .and_then(|registry| registry.dispatch_fn(&message.key));
                let Some(dispatch) = dispatch else {
                    warn!(
                        "Dialogue message key {} is not registered or message registry is unavailable",
                        message.key
                    );
                    continue;
                };

                if let Err(err) = dispatch(world, &message) {
                    error!(
                        "Failed to dispatch dialogue message {}: {}",
                        message.key, err
                    );
                    continue;
                }
                world
                    .resource_mut::<Messages<AdvanceDialogue>>()
                    .write(AdvanceDialogue {
                        entity: event.entity,
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
/// ```rust,ignore
/// use bevy::prelude::*;
/// use funkus_dialogue_core::runtime::setup_dialogue_systems;
///
/// fn main() {
///     let mut app = App::new();
///     setup_dialogue_systems(&mut app);
///     // ... add other app configuration
///     app.run();
/// }
/// ```
pub fn setup_dialogue_systems(app: &mut App) {
    app.configure_sets(Update, DialogueSystemSet)
        .configure_sets(PostUpdate, DialogueSystemSet)
        .add_systems(
            Update,
            (
                update_dialogue_runners,
                handle_stop_dialogue_events,
                handle_advance_dialogue_events,
                handle_select_dialogue_events,
            )
                .chain()
                .in_set(DialogueSystemSet),
        )
        .add_systems(
            PostUpdate,
            (handle_start_dialogue_events, apply_dialogue_actions)
                .chain()
                .in_set(DialogueSystemSet),
        );
    app.init_resource::<DialogueActionCursor>();
}

#[cfg(test)]
mod tests {
    use bevy::ecs::message::{MessageCursor, Messages};
    use bevy::prelude::*;

    use crate::asset::DialogueAsset;
    use crate::events::{
        AdvanceDialogue, DialogueChoiceMade, DialogueEnded, DialogueNodeActivated, DialogueStarted,
        SelectDialogueChoice, StartDialogue, StopDialogue,
    };
    use crate::graph::{ConnectionData, DialogueGraph, DialogueNode};
    use crate::registry::{
        DialogueEffect, DialogueMessageCall, DialogueMessageRegistryPlugin, DialogueOperation,
        DialogueRegistry, DialogueValue,
    };
    use crate::runtime::{DialogueRunner, DialogueState, setup_dialogue_systems};

    #[derive(Resource, Reflect, crate::DialogueResource, Default)]
    #[dialogue(key = "runtime_test")]
    struct RuntimeTestState {
        value: i32,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Reflect)]
    enum RuntimeTestTone {
        Calm,
        Urgent,
    }

    #[derive(Message, Clone, Debug, PartialEq, Reflect, crate::DialogueMessage)]
    #[dialogue(key = "runtime_test.message")]
    struct RuntimeTestMessage {
        amount: i32,
        label: String,
        tone: RuntimeTestTone,
    }

    #[derive(Resource)]
    struct DeferredStartHandle(Handle<DialogueAsset>);

    #[derive(Resource, Default)]
    struct DeferredStartSent(bool);

    fn queue_start_with_deferred_spawn(
        mut commands: Commands,
        mut sent: ResMut<DeferredStartSent>,
        handle: Res<DeferredStartHandle>,
        mut start_events: MessageWriter<StartDialogue>,
    ) {
        if sent.0 {
            return;
        }
        sent.0 = true;

        let entity = commands
            .spawn((
                Name::new("Deferred Start Runner"),
                DialogueRunner::default(),
            ))
            .id();
        start_events.write(StartDialogue {
            entity,
            dialogue_handle: handle.0.clone(),
        });
    }

    fn init_runtime_test_app() -> App {
        let mut app = App::new();
        app.init_resource::<Assets<DialogueAsset>>()
            .init_resource::<Time>()
            .init_resource::<DialogueRegistry>()
            .init_resource::<AppTypeRegistry>()
            .add_plugins(DialogueMessageRegistryPlugin)
            .add_message::<DialogueStarted>()
            .add_message::<DialogueEnded>()
            .add_message::<DialogueNodeActivated>()
            .add_message::<DialogueChoiceMade>()
            .add_message::<AdvanceDialogue>()
            .add_message::<SelectDialogueChoice>()
            .add_message::<StartDialogue>()
            .add_message::<StopDialogue>();
        setup_dialogue_systems(&mut app);
        app
    }

    #[test]
    fn start_and_advance_emit_node_activated_messages() {
        let mut app = init_runtime_test_app();

        let mut graph = DialogueGraph::new();
        let start = graph.add_node(DialogueNode::text("Start"));
        let next = graph.add_node(DialogueNode::text("Next"));
        graph
            .connect(start, next, ConnectionData::new(None))
            .expect("connect test nodes");
        graph.set_start_node(start).expect("set start");

        let handle = app
            .world_mut()
            .resource_mut::<Assets<DialogueAsset>>()
            .add(DialogueAsset::new(graph));

        let entity = app.world_mut().spawn(DialogueRunner::default()).id();
        app.world_mut()
            .resource_mut::<Messages<StartDialogue>>()
            .write(StartDialogue {
                entity,
                dialogue_handle: handle,
            });

        let mut activated_cursor = MessageCursor::<DialogueNodeActivated>::default();

        app.update();
        let activated_on_start: Vec<DialogueNodeActivated> = {
            let messages = app.world().resource::<Messages<DialogueNodeActivated>>();
            activated_cursor.read(messages).cloned().collect()
        };
        assert_eq!(activated_on_start.len(), 1);
        assert_eq!(activated_on_start[0].entity, entity);
        assert_eq!(activated_on_start[0].node_id, start);

        app.world_mut()
            .resource_mut::<Messages<AdvanceDialogue>>()
            .write(AdvanceDialogue { entity });
        app.update();

        let activated_on_advance: Vec<DialogueNodeActivated> = {
            let messages = app.world().resource::<Messages<DialogueNodeActivated>>();
            activated_cursor.read(messages).cloned().collect()
        };
        assert_eq!(activated_on_advance.len(), 1);
        assert_eq!(activated_on_advance[0].entity, entity);
        assert_eq!(activated_on_advance[0].node_id, next);
    }

    #[test]
    fn start_dialogue_is_noop_when_asset_is_not_loaded() {
        let mut app = init_runtime_test_app();

        let mut graph = DialogueGraph::new();
        let start = graph.add_node(DialogueNode::text("Existing"));
        graph.set_start_node(start).expect("set start");
        let existing_handle = app
            .world_mut()
            .resource_mut::<Assets<DialogueAsset>>()
            .add(DialogueAsset::new(graph));

        let entity = app
            .world_mut()
            .spawn(DialogueRunner::new(existing_handle.clone()))
            .id();

        app.world_mut()
            .resource_mut::<Messages<StartDialogue>>()
            .write(StartDialogue {
                entity,
                dialogue_handle: Handle::<DialogueAsset>::default(),
            });

        let mut started_cursor = MessageCursor::<DialogueStarted>::default();
        let mut activated_cursor = MessageCursor::<DialogueNodeActivated>::default();

        app.update();

        let started: Vec<DialogueStarted> = {
            let messages = app.world().resource::<Messages<DialogueStarted>>();
            started_cursor.read(messages).cloned().collect()
        };
        assert!(started.is_empty());

        let activated: Vec<DialogueNodeActivated> = {
            let messages = app.world().resource::<Messages<DialogueNodeActivated>>();
            activated_cursor.read(messages).cloned().collect()
        };
        assert!(activated.is_empty());

        let runner = app
            .world()
            .get::<DialogueRunner>(entity)
            .expect("runner should still exist");
        assert_eq!(runner.dialogue_handle, existing_handle);
        assert_eq!(runner.state, DialogueState::Inactive);
        assert!(runner.current_node_id.is_none());
    }

    #[test]
    fn effect_nodes_mutate_registered_resources_and_queue_advance() {
        let mut app = init_runtime_test_app();
        app.insert_resource(RuntimeTestState { value: 2 });

        {
            let app_type_registry = app.world_mut().resource_mut::<AppTypeRegistry>();
            let mut type_registry = app_type_registry.write();
            type_registry.register::<RuntimeTestState>();
        }

        app.world_mut()
            .resource_mut::<DialogueRegistry>()
            .register_reflected_resource(
                <RuntimeTestState as bevy::reflect::Typed>::type_info(),
                <RuntimeTestState as crate::registry::DialogueResource>::resource_key(),
            );

        let mut graph = DialogueGraph::new();
        let start = graph.add_node(DialogueNode::effect(DialogueEffect {
            key: "runtime_test.value".to_string(),
            op: DialogueOperation::Add,
            value: DialogueValue::Int(5),
        }));
        graph.set_start_node(start).expect("set start");

        let handle = app
            .world_mut()
            .resource_mut::<Assets<DialogueAsset>>()
            .add(DialogueAsset::new(graph));
        let entity = app.world_mut().spawn(DialogueRunner::default()).id();

        app.world_mut()
            .resource_mut::<Messages<StartDialogue>>()
            .write(StartDialogue {
                entity,
                dialogue_handle: handle,
            });

        app.update();

        assert_eq!(app.world().resource::<RuntimeTestState>().value, 7);

        let mut advance_cursor = MessageCursor::<AdvanceDialogue>::default();
        let advances: Vec<AdvanceDialogue> = {
            let messages = app.world().resource::<Messages<AdvanceDialogue>>();
            advance_cursor.read(messages).cloned().collect()
        };
        assert_eq!(advances.len(), 1);
        assert_eq!(advances[0].entity, entity);
    }

    #[test]
    fn unknown_effect_keys_do_not_mutate_resources_or_auto_advance() {
        let mut app = init_runtime_test_app();
        app.insert_resource(RuntimeTestState { value: 10 });

        let mut graph = DialogueGraph::new();
        let start = graph.add_node(DialogueNode::effect(DialogueEffect {
            key: "runtime_test.missing".to_string(),
            op: DialogueOperation::Set,
            value: DialogueValue::Int(99),
        }));
        graph.set_start_node(start).expect("set start");

        let handle = app
            .world_mut()
            .resource_mut::<Assets<DialogueAsset>>()
            .add(DialogueAsset::new(graph));
        let entity = app.world_mut().spawn(DialogueRunner::default()).id();

        app.world_mut()
            .resource_mut::<Messages<StartDialogue>>()
            .write(StartDialogue {
                entity,
                dialogue_handle: handle,
            });

        app.update();

        assert_eq!(app.world().resource::<RuntimeTestState>().value, 10);

        let mut advance_cursor = MessageCursor::<AdvanceDialogue>::default();
        let advances: Vec<AdvanceDialogue> = {
            let messages = app.world().resource::<Messages<AdvanceDialogue>>();
            advance_cursor.read(messages).cloned().collect()
        };
        assert!(advances.is_empty());
    }

    #[test]
    fn message_nodes_dispatch_registered_bevy_messages_and_queue_advance() {
        let mut app = init_runtime_test_app();

        let mut graph = DialogueGraph::new();
        let start = graph.add_node(DialogueNode::message(
            DialogueMessageCall::new("runtime_test.message")
                .with_param("amount", DialogueValue::Int(12))
                .with_param("label", DialogueValue::String("hello".to_string()))
                .with_param("tone", DialogueValue::Enum("Urgent".to_string())),
        ));
        graph.set_start_node(start).expect("set start");

        let handle = app
            .world_mut()
            .resource_mut::<Assets<DialogueAsset>>()
            .add(DialogueAsset::new(graph));
        let entity = app.world_mut().spawn(DialogueRunner::default()).id();

        app.world_mut()
            .resource_mut::<Messages<StartDialogue>>()
            .write(StartDialogue {
                entity,
                dialogue_handle: handle,
            });

        app.update();

        let mut message_cursor = MessageCursor::<RuntimeTestMessage>::default();
        let sent: Vec<RuntimeTestMessage> = {
            let messages = app.world().resource::<Messages<RuntimeTestMessage>>();
            message_cursor.read(messages).cloned().collect()
        };
        assert_eq!(sent.len(), 1);
        assert_eq!(
            sent[0],
            RuntimeTestMessage {
                amount: 12,
                label: "hello".to_string(),
                tone: RuntimeTestTone::Urgent,
            }
        );

        let mut advance_cursor = MessageCursor::<AdvanceDialogue>::default();
        let advances: Vec<AdvanceDialogue> = {
            let messages = app.world().resource::<Messages<AdvanceDialogue>>();
            advance_cursor.read(messages).cloned().collect()
        };
        assert_eq!(advances.len(), 1);
        assert_eq!(advances[0].entity, entity);
    }

    #[test]
    fn start_dialogue_from_deferred_spawn_starts_in_same_frame() {
        let mut app = init_runtime_test_app();

        let mut graph = DialogueGraph::new();
        let start = graph.add_node(DialogueNode::text("Start"));
        graph.set_start_node(start).expect("set start");

        let handle = app
            .world_mut()
            .resource_mut::<Assets<DialogueAsset>>()
            .add(DialogueAsset::new(graph));

        app.insert_resource(DeferredStartHandle(handle));
        app.init_resource::<DeferredStartSent>();
        app.add_systems(Update, queue_start_with_deferred_spawn);

        let mut started_cursor = MessageCursor::<DialogueStarted>::default();

        app.update();
        let started_after_first_update: Vec<DialogueStarted> = {
            let messages = app.world().resource::<Messages<DialogueStarted>>();
            started_cursor.read(messages).cloned().collect()
        };
        assert_eq!(started_after_first_update.len(), 1);
        assert_eq!(started_after_first_update[0].start_node_id, start);
    }
}
