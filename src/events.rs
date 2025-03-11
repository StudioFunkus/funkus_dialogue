//! # Events for dialogue system interaction.
//! 
//! This module defines the events used for interacting with the dialogue system,
//! both from game code to the dialogue system and from the dialogue system to game code.
//! 
//! ## Event Types
//! 
//! The events are divided into two categories:
//! 
//! 1. **Command Events** - Sent to the dialogue system to request actions:
//!    - `StartDialogue` - Start a dialogue
//!    - `StopDialogue` - Stop a dialogue
//!    - `AdvanceDialogue` - Move to the next node
//!    - `SelectDialogueChoice` - Select a choice
//! 
//! 2. **Notification Events** - Sent by the dialogue system to notify about state changes:
//!    - `DialogueStarted` - A dialogue has started
//!    - `DialogueEnded` - A dialogue has ended
//!    - `DialogueNodeActivated` - A node has been activated
//!    - `DialogueChoiceMade` - A choice has been selected
//! 
//! ## Usage Example
//! 
//! ```rust
//! use bevy::prelude::*;
//! use funkus_dialogue::{
//!     StartDialogue, StopDialogue, AdvanceDialogue, SelectDialogueChoice,
//!     DialogueStarted, DialogueEnded,
//! };
//! 
//! fn dialogue_control_system(
//!     mut start_events: EventWriter<StartDialogue>,
//!     mut advance_events: EventWriter<AdvanceDialogue>,
//!     mut dialogue_ended_reader: EventReader<DialogueEnded>,
//!     keyboard: Res<Input<KeyCode>>,
//!     entity: Query<Entity, With<Player>>,
//! ) {
//!     // React to dialogue ended events
//!     for event in dialogue_ended_reader.read() {
//!         println!("Dialogue ended: {:?}", event.entity);
//!     }
//!     
//!     // Start dialogue when talking to NPC
//!     if keyboard.just_pressed(KeyCode::E) {
//!         if let Ok(player) = entity.get_single() {
//!             // Request to start a dialogue
//!             start_events.send(StartDialogue {
//!                 entity: player,
//!                 dialogue_handle: Handle::default(), // Use actual handle
//!             });
//!         }
//!     }
//!     
//!     // Advance dialogue when space is pressed
//!     if keyboard.just_pressed(KeyCode::Space) {
//!         if let Ok(player) = entity.get_single() {
//!             advance_events.send(AdvanceDialogue {
//!                 entity: player,
//!             });
//!         }
//!     }
//! }
//! ```

use bevy::prelude::*;

use crate::graph::NodeId;

/// Event sent when a dialogue starts.
/// 
/// This event is emitted by the dialogue system when a dialogue begins. 
/// It can be used by game systems to react to the start of a conversation.
/// 
/// # Fields
/// 
/// * `entity` - Entity running the dialogue
/// * `start_node_id` - ID of the start node
/// 
/// # Example
/// 
/// ```rust
/// use bevy::prelude::*;
/// use funkus_dialogue::DialogueStarted;
/// 
/// fn on_dialogue_start(mut dialogue_events: EventReader<DialogueStarted>) {
///     for event in dialogue_events.read() {
///         println!("Dialogue started with entity {:?}", event.entity);
///         // Play dialogue start sound, change camera, etc.
///     }
/// }
/// ```
#[derive(Event, Debug, Clone)]
pub struct DialogueStarted {
    /// Entity running the dialogue
    pub entity: Entity,
    /// ID of the start node
    pub start_node_id: NodeId,
}

/// Event sent when a dialogue node is activated.
/// 
/// This event is emitted whenever the dialogue moves to a new node.
/// It can be used to track dialogue progress or trigger game events
/// based on specific nodes.
/// 
/// # Fields
/// 
/// * `entity` - Entity running the dialogue
/// * `node_id` - ID of the activated node
/// 
/// # Example
/// 
/// ```rust
/// use bevy::prelude::*;
/// use funkus_dialogue::{DialogueNodeActivated, NodeId};
/// 
/// fn track_node_activation(mut node_events: EventReader<DialogueNodeActivated>) {
///     for event in node_events.read() {
///         // Trigger game events based on specific nodes
///         if event.node_id == NodeId(5) {
///             println!("Special node activated!");
///             // Trigger special game event
///         }
///     }
/// }
/// ```
#[derive(Event, Debug, Clone)]
pub struct DialogueNodeActivated {
    /// Entity running the dialogue
    pub entity: Entity,
    /// ID of the activated node
    pub node_id: NodeId,
}

/// Event sent when a player makes a choice in a dialogue.
/// 
/// This event is emitted when the player selects a choice in a choice node.
/// It can be used to track player decisions or trigger game events based on choices.
/// 
/// # Fields
/// 
/// * `entity` - Entity running the dialogue
/// * `node_id` - ID of the choice node
/// * `choice_index` - Index of the selected choice
/// 
/// # Example
/// 
/// ```rust
/// use bevy::prelude::*;
/// use funkus_dialogue::DialogueChoiceMade;
/// 
/// fn track_player_choices(mut choice_events: EventReader<DialogueChoiceMade>) {
///     for event in choice_events.read() {
///         println!(
///             "Entity {:?} selected choice {} in node {:?}", 
///             event.entity, event.choice_index, event.node_id
///         );
///         // Track choices for quest or relationship systems
///     }
/// }
/// ```
#[derive(Event, Debug, Clone)]
pub struct DialogueChoiceMade {
    /// Entity running the dialogue
    pub entity: Entity,
    /// ID of the choice node
    pub node_id: NodeId,
    /// Index of the selected choice
    pub choice_index: usize,
}

/// Event sent when a dialogue ends.
/// 
/// This event is emitted when a dialogue completes, either by reaching
/// an end node or by being forcibly stopped. It can be used to reset
/// game state or trigger post-dialogue actions.
/// 
/// # Fields
/// 
/// * `entity` - Entity running the dialogue
/// * `normal_exit` - Whether the dialogue ended normally (as opposed to being forcibly stopped)
/// 
/// # Example
/// 
/// ```rust
/// use bevy::prelude::*;
/// use funkus_dialogue::DialogueEnded;
/// 
/// fn on_dialogue_end(mut end_events: EventReader<DialogueEnded>) {
///     for event in end_events.read() {
///         if event.normal_exit {
///             println!("Dialogue completed normally");
///         } else {
///             println!("Dialogue was interrupted");
///         }
///         // Return camera to normal, resume gameplay, etc.
///     }
/// }
/// ```
#[derive(Event, Debug, Clone)]
pub struct DialogueEnded {
    /// Entity running the dialogue
    pub entity: Entity,
    /// Whether the dialogue ended normally (as opposed to being forcibly stopped)
    pub normal_exit: bool,
}

/// Event to request advancing the dialogue.
/// 
/// Send this event to move the dialogue to the next node.
/// For text nodes, this advances to the next node in the sequence.
/// For choice nodes, this confirms the selected choice and moves to the target node.
/// 
/// # Fields
/// 
/// * `entity` - Entity running the dialogue
/// 
/// # Example
/// 
/// ```rust
/// use bevy::prelude::*;
/// use funkus_dialogue::AdvanceDialogue;
/// 
/// fn advance_on_space(
///     keyboard: Res<Input<KeyCode>>,
///     dialogue_entities: Query<Entity, With<DialogueRunner>>,
///     mut advance_events: EventWriter<AdvanceDialogue>,
/// ) {
///     if keyboard.just_pressed(KeyCode::Space) {
///         for entity in dialogue_entities.iter() {
///             advance_events.send(AdvanceDialogue {
///                 entity,
///             });
///         }
///     }
/// }
/// ```
#[derive(Event, Debug, Clone)]
pub struct AdvanceDialogue {
    /// Entity running the dialogue
    pub entity: Entity,
}

/// Event to request selecting a choice.
/// 
/// Send this event to select a choice in a choice node.
/// The choice isn't confirmed until an AdvanceDialogue event is sent.
/// 
/// # Fields
/// 
/// * `entity` - Entity running the dialogue
/// * `choice_index` - Index of the choice to select
/// 
/// # Example
/// 
/// ```rust
/// use bevy::prelude::*;
/// use funkus_dialogue::{SelectDialogueChoice, DialogueRunner, DialogueState};
/// 
/// fn select_choice_with_number_keys(
///     keyboard: Res<Input<KeyCode>>,
///     dialogue_query: Query<(Entity, &DialogueRunner)>,
///     mut select_events: EventWriter<SelectDialogueChoice>,
/// ) {
///     for (entity, runner) in dialogue_query.iter() {
///         if runner.state == DialogueState::WaitingForChoice {
///             // Check for number key presses (1-9)
///             for i in 0..9 {
///                 let key = match i {
///                     0 => KeyCode::Key1,
///                     1 => KeyCode::Key2,
///                     2 => KeyCode::Key3,
///                     // ... and so on
///                     _ => continue,
///                 };
///                 
///                 if keyboard.just_pressed(key) {
///                     select_events.send(SelectDialogueChoice {
///                         entity,
///                         choice_index: i,
///                     });
///                 }
///             }
///         }
///     }
/// }
/// ```
#[derive(Event, Debug, Clone)]
pub struct SelectDialogueChoice {
    /// Entity running the dialogue
    pub entity: Entity,
    /// Index of the choice to select
    pub choice_index: usize,
}

/// Event to request starting a dialogue.
/// 
/// Send this event to start a dialogue on an entity.
/// The entity should have a DialogueRunner component, or one will be added.
/// 
/// # Fields
/// 
/// * `entity` - Entity to attach the dialogue runner to
/// * `dialogue_handle` - Handle to the dialogue asset
/// 
/// # Example
/// 
/// ```rust
/// use bevy::prelude::*;
/// use funkus_dialogue::{StartDialogue, DialogueAsset};
/// 
/// fn start_dialogue_on_interact(
///     keyboard: Res<Input<KeyCode>>,
///     asset_server: Res<AssetServer>,
///     player_query: Query<Entity, With<Player>>,
///     npc_query: Query<&Interactable, With<Npc>>,
///     mut start_events: EventWriter<StartDialogue>,
/// ) {
///     if keyboard.just_pressed(KeyCode::E) && player_query.get_single().is_ok() {
///         let player = player_query.single();
///         
///         // Check if player is near an interactable NPC
///         for interactable in npc_query.iter() {
///             if interactable.is_in_range {
///                 // Load dialogue asset for this NPC
///                 let dialogue_handle = asset_server.load("dialogues/npc.dialogue.json");
///                 
///                 // Start the dialogue on the player entity
///                 start_events.send(StartDialogue {
///                     entity: player,
///                     dialogue_handle,
///                 });
///             }
///         }
///     }
/// }
/// ```
#[derive(Event, Debug, Clone)]
pub struct StartDialogue {
    /// Entity to attach the dialogue runner to
    pub entity: Entity,
    /// Handle to the dialogue asset
    pub dialogue_handle: Handle<crate::asset::DialogueAsset>,
}

/// Event to request stopping a dialogue.
/// 
/// Send this event to forcibly stop a dialogue that's in progress.
/// This will reset the DialogueRunner to an inactive state.
/// 
/// # Fields
/// 
/// * `entity` - Entity running the dialogue
/// 
/// # Example
/// 
/// ```rust
/// use bevy::prelude::*;
/// use funkus_dialogue::{StopDialogue, DialogueRunner};
/// 
/// fn stop_dialogue_on_escape(
///     keyboard: Res<Input<KeyCode>>,
///     dialogue_query: Query<Entity, With<DialogueRunner>>,
///     mut stop_events: EventWriter<StopDialogue>,
/// ) {
///     if keyboard.just_pressed(KeyCode::Escape) {
///         for entity in dialogue_query.iter() {
///             stop_events.send(StopDialogue {
///                 entity,
///             });
///         }
///     }
/// }
/// ```
#[derive(Event, Debug, Clone)]
pub struct StopDialogue {
    /// Entity running the dialogue
    pub entity: Entity,
}