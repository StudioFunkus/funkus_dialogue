//! # Dialogue runner component and state management.
//!
//! This module defines the DialogueRunner component, which processes dialogues at runtime,
//! and the DialogueState enum, which represents the current state of a dialogue.

use bevy::prelude::*;
use std::collections::HashMap;

use crate::asset::DialogueAsset;
use crate::error::{DialogueError, DialogueResult};
use crate::graph::{DialogueNode, NodeId};

/// Current state of a dialogue.
///
/// This enum represents the possible states that a dialogue can be in
/// during runtime processing. The state determines what actions can be
/// taken (advancing, selecting choices) and how the dialogue is displayed.
///
/// # Variants
///
/// * `Inactive` - Dialogue is not currently running
/// * `ShowingText` - Dialogue is displaying text
/// * `WaitingForChoice` - Dialogue is waiting for player to select a choice
/// * `ChoiceSelected(usize)` - Player has selected a choice, ready to advance
/// * `Finished` - Dialogue has reached an end node
/// * `Error(String)` - Dialogue encountered an error
///
/// # State Transitions
///
/// The possible state transitions are:
///
/// - `Inactive` -> `ShowingText` or `WaitingForChoice` (when starting)
/// - `ShowingText` -> `ShowingText` or `WaitingForChoice` or `Finished` (when advancing)
/// - `WaitingForChoice` -> `ChoiceSelected` (when selecting)
/// - `ChoiceSelected` -> `ChoiceSelected` (when selecting a different choice)
/// - `ChoiceSelected` -> `ShowingText` or `WaitingForChoice` or `Finished` (when advancing)
/// - Any state -> `Inactive` (when stopping)
/// - Any state -> `Error` (when an error occurs)
#[derive(Debug, Clone, Reflect, PartialEq, Eq)]
pub enum DialogueState {
    /// Dialogue is not currently running
    Inactive,
    /// Dialogue is displaying text
    ShowingText,
    /// Dialogue is processing a non-visual action node (effect/message)
    ProcessingEffect,
    /// Dialogue is waiting for player to select a choice
    WaitingForChoice,
    /// Player has selected a choice, ready to advance to next node
    ChoiceSelected(usize),
    /// Dialogue has reached an end node
    Finished,
    /// Dialogue encountered an error
    Error(String),
}

impl DialogueState {
    /// Get a string representation of the state for error messages
    ///
    /// # Returns
    ///
    /// A string name for the current state
    pub fn name(&self) -> String {
        match self {
            DialogueState::Inactive => "Inactive".to_string(),
            DialogueState::ShowingText => "ShowingText".to_string(),
            DialogueState::ProcessingEffect => "ProcessingEffect".to_string(),
            DialogueState::WaitingForChoice => "WaitingForChoice".to_string(),
            DialogueState::ChoiceSelected(_) => "ChoiceSelected".to_string(),
            DialogueState::Finished => "Finished".to_string(),
            DialogueState::Error(_) => "Error".to_string(),
        }
    }

    /// Check if this state can transition to showing the next node
    ///
    /// # Returns
    ///
    /// `true` if the dialogue can advance from this state, `false` otherwise
    pub fn can_advance(&self) -> bool {
        matches!(
            self,
            DialogueState::ShowingText
                | DialogueState::ProcessingEffect
                | DialogueState::ChoiceSelected(_)
        )
    }

    /// Check if a choice can be selected in this state
    ///
    /// # Returns
    ///
    /// `true` if a choice can be selected in this state, `false` otherwise
    pub fn can_select_choice(&self) -> bool {
        matches!(
            self,
            DialogueState::WaitingForChoice | DialogueState::ChoiceSelected(_)
        )
    }
}

/// Component that processes and manages a dialogue.
///
/// DialogueRunner is the core component for dialogue runtime processing.
/// When attached to an entity, it allows that entity to run a dialogue,
/// tracking the current state, processing player input, and managing
/// transitions between nodes.
///
/// # Fields
///
/// * `dialogue_handle` - Handle to the dialogue asset
/// * `current_node_id` - ID of the current active node
/// * `state` - Current state of the dialogue
/// * `auto_advance` - Whether the dialogue should auto-advance after text nodes
/// * `auto_advance_time` - Time to wait for auto-advance (in seconds)
/// * `auto_advance_timer` - Timer for auto-advance
/// * `variables` - Simple variable storage
///
/// # Auto-Advance Feature
///
/// The DialogueRunner includes an auto-advance feature that can automatically advance
/// text nodes after a specified time. This is useful for cutscenes or situations where
/// player interaction should be minimized.
///
/// - `auto_advance`: Enable/disable the auto-advance feature
/// - `auto_advance_time`: Time in seconds to wait before advancing
/// - `auto_advance_timer`: Internal timer that tracks elapsed time
///
/// Auto-advance only applies to text nodes and is ignored for choice nodes.
/// The timer is reset whenever the dialogue advances to a new node.
///
/// # Example
///
/// ```rust,ignore
/// use bevy::prelude::*;
/// use funkus_dialogue_core::DialogueRunner;
///
/// fn setup(mut commands: Commands) {
///     // Create an entity with a dialogue runner
///     commands.spawn((
///         Name::new("NPC Dialogue"),
///         DialogueRunner::default(),
///     ));
/// }
/// ```
#[derive(Component, Debug)]
pub struct DialogueRunner {
    /// Handle to the dialogue asset
    pub dialogue_handle: Handle<DialogueAsset>,
    /// ID of the current active node
    pub current_node_id: Option<NodeId>,
    /// Current state of the dialogue
    pub state: DialogueState,
    /// Whether the dialogue should auto-advance after text nodes
    pub auto_advance: bool,
    /// Time to wait for auto-advance (in seconds)
    pub auto_advance_time: f32,
    /// Timer for auto-advance
    pub auto_advance_timer: Timer,
    /// Simple variable storage (to be expanded later)
    pub variables: HashMap<String, String>,
}

impl Default for DialogueRunner {
    fn default() -> Self {
        Self {
            dialogue_handle: Handle::default(),
            current_node_id: None,
            state: DialogueState::Inactive,
            auto_advance: false,
            auto_advance_time: 2.0,
            auto_advance_timer: Timer::from_seconds(2.0, TimerMode::Once),
            variables: HashMap::new(),
        }
    }
}

impl DialogueRunner {
    /// Creates a new dialogue runner for the given dialogue asset.
    ///
    /// # Parameters
    ///
    /// * `dialogue_handle` - Handle to the dialogue asset to run
    ///
    /// # Returns
    ///
    /// A new DialogueRunner configured to run the specified dialogue
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use bevy::prelude::*;
    /// use funkus_dialogue_core::DialogueRunner;
    ///
    /// fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    ///     let dialogue_handle = asset_server.load("dialogue/example.dialogue.json");
    ///     
    ///     commands.spawn(DialogueRunner::new(dialogue_handle));
    /// }
    /// ```
    pub fn new(dialogue_handle: Handle<DialogueAsset>) -> Self {
        Self {
            dialogue_handle,
            ..Default::default()
        }
    }

    /// Starts the dialogue from the beginning.
    ///
    /// This method initializes the dialogue runner with the start node
    /// from the provided dialogue asset and sets the appropriate initial state.
    ///
    /// # Parameters
    ///
    /// * `dialogue` - The dialogue asset to start
    ///
    /// # Returns
    ///
    /// [`DialogueResult<()>`](crate::error::DialogueResult) indicating whether startup
    /// succeeded. Errors report missing start nodes or start nodes that no longer exist
    /// in the graph.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// # use bevy::prelude::*;
    /// # use funkus_dialogue_core::{DialogueAsset, DialogueRunner};
    /// #
    /// fn start_dialogue(
    ///     dialogue_assets: Res<Assets<DialogueAsset>>,
    ///     mut dialogue_query: Query<&mut DialogueRunner>,
    /// ) {
    ///     for mut runner in dialogue_query.iter_mut() {
    ///         if let Some(dialogue) = dialogue_assets.get(&runner.dialogue_handle) {
    ///             if let Err(err) = runner.start(dialogue) {
    ///                 eprintln!("Failed to start dialogue: {}", err);
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    pub fn start(&mut self, dialogue: &DialogueAsset) -> DialogueResult<()> {
        self.current_node_id = None;
        self.state = DialogueState::Inactive;

        let start_id = dialogue
            .graph
            .start_node
            .ok_or_else(|| DialogueError::GraphError("Dialogue has no start node".to_string()))?;

        let start_node = dialogue
            .graph
            .get_node(start_id)
            .ok_or(DialogueError::NodeNotFound(start_id))?;

        self.current_node_id = Some(start_id);
        self.state = match start_node {
            DialogueNode::Text { .. } => DialogueState::ShowingText,
            DialogueNode::Choice { .. } => DialogueState::WaitingForChoice,
            DialogueNode::Effect { .. } | DialogueNode::Message { .. } => {
                DialogueState::ProcessingEffect
            }
        };

        self.auto_advance_timer.reset();
        Ok(())
    }

    /// Advances to the next node in the dialogue.
    ///
    /// This method processes the current node and transitions to the next node
    /// based on the dialogue structure and player choices.
    ///
    /// # Parameters
    ///
    /// * `dialogue` - The dialogue asset being processed
    ///
    /// # Returns
    ///
    /// A result indicating success or an error with details
    ///
    /// # Errors
    ///
    /// This method can return various errors such as:
    ///
    /// - `InvalidStateTransition` - Cannot advance from the current state
    /// - `NoCurrentNode` - No current node is active
    /// - `NodeNotFound` - The current node ID doesn't exist in the dialogue
    /// - `NoChoiceSelected` - Trying to advance from a choice node without a selection
    /// - `NextNodeNotFound` - The target node doesn't exist
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// # use bevy::prelude::*;
    /// # use funkus_dialogue_core::{DialogueAsset, DialogueRunner, DialogueState};
    /// #
    /// fn advance_dialogue(
    ///     dialogue_assets: Res<Assets<DialogueAsset>>,
    ///     mut dialogue_query: Query<&mut DialogueRunner>,
    /// ) {
    ///     for mut runner in dialogue_query.iter_mut() {
    ///         if runner.state == DialogueState::ShowingText {
    ///             if let Some(dialogue) = dialogue_assets.get(&runner.dialogue_handle) {
    ///                 if let Err(err) = runner.advance(dialogue) {
    ///                     eprintln!("Error advancing dialogue: {}", err);
    ///                 }
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    pub fn advance(&mut self, dialogue: &DialogueAsset) -> DialogueResult<()> {
        // Check if we can advance in the current state
        if !self.state.can_advance() {
            return Err(DialogueError::InvalidStateTransition {
                from: self.state.name(),
                action: "advance".to_string(),
            });
        }

        let current_id = self.current_node_id.ok_or(DialogueError::NoCurrentNode)?;

        let current_node = dialogue
            .graph
            .get_node(current_id)
            .ok_or(DialogueError::NodeNotFound(current_id))?;

        match current_node {
            DialogueNode::Text { .. }
            | DialogueNode::Effect { .. }
            | DialogueNode::Message { .. } => {
                // Get connections from the graph instead of the node
                let connections = dialogue.graph.get_connected_nodes(current_id);

                // A text node typically has 0 or 1 connections
                if connections.is_empty() {
                    // End of dialogue
                    self.state = DialogueState::Finished;
                    return Ok(());
                }

                // Move to the next node
                let next_id = connections[0].0;
                self.current_node_id = Some(next_id);

                // Update state based on the next node type
                if let Some(next_node) = dialogue.graph.get_node(next_id) {
                    match next_node {
                        DialogueNode::Text { .. } => self.state = DialogueState::ShowingText,
                        DialogueNode::Choice { .. } => self.state = DialogueState::WaitingForChoice,
                        DialogueNode::Effect { .. } | DialogueNode::Message { .. } => {
                            self.state = DialogueState::ProcessingEffect;
                        }
                    }
                } else {
                    return Err(DialogueError::NextNodeNotFound(next_id));
                }
            }
            DialogueNode::Choice { .. } => {
                // For choice nodes, we need a selected choice
                // Check if we're in the ChoiceSelected state
                let choice_index = match self.state {
                    DialogueState::ChoiceSelected(index) => index,
                    _ => {
                        return Err(DialogueError::NoChoiceSelected);
                    }
                };

                // Get connections from the graph
                let connections = dialogue.graph.get_connected_nodes(current_id);

                if connections.is_empty() {
                    return Err(DialogueError::GraphError(format!(
                        "Choice node {:?} has no outgoing connections",
                        current_id
                    )));
                }

                if choice_index >= connections.len() {
                    return Err(DialogueError::InvalidChoiceIndex(
                        choice_index,
                        connections.len().saturating_sub(1),
                    ));
                }

                // Move to the selected choice's target node
                let next_id = connections[choice_index].0;
                self.current_node_id = Some(next_id);

                // Update state based on the next node type
                if let Some(next_node) = dialogue.graph.get_node(next_id) {
                    match next_node {
                        DialogueNode::Text { .. } => self.state = DialogueState::ShowingText,
                        DialogueNode::Choice { .. } => self.state = DialogueState::WaitingForChoice,
                        DialogueNode::Effect { .. } | DialogueNode::Message { .. } => {
                            self.state = DialogueState::ProcessingEffect;
                        }
                    }
                } else {
                    return Err(DialogueError::NextNodeNotFound(next_id));
                }
            }
        }

        // Reset timer for auto-advance
        self.auto_advance_timer.reset();

        Ok(())
    }

    /// Selects a choice option.
    ///
    /// This method updates the dialogue state to reflect that a choice has been selected.
    /// The dialogue state changes to `DialogueState::ChoiceSelected(choice_index)`.
    ///
    /// # Parameters
    ///
    /// * `choice_index` - The index of the choice to select
    ///
    /// # Returns
    ///
    /// A result indicating success or an error with details
    ///
    /// # Errors
    ///
    /// This method can return `InvalidStateTransition` if a choice cannot be
    /// selected in the current state.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// # use bevy::prelude::*;
    /// # use funkus_dialogue_core::{DialogueRunner, DialogueState};
    /// #
    /// fn select_choice(
    ///     mut dialogue_query: Query<&mut DialogueRunner>,
    ///     keyboard_input: Res<ButtonInput<KeyCode>>,
    /// ) {
    ///     for mut runner in dialogue_query.iter_mut() {
    ///         if runner.state == DialogueState::WaitingForChoice {
    ///             // Select the first choice when '1' is pressed
    ///             if keyboard_input.just_pressed(KeyCode::Digit1) {
    ///                 if let Err(err) = runner.select_choice(0) {
    ///                     eprintln!("Error selecting choice: {}", err);
    ///                 }
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    pub fn select_choice(&mut self, choice_index: usize) -> DialogueResult<()> {
        // Check if we can select a choice in the current state
        if !self.state.can_select_choice() {
            return Err(DialogueError::InvalidStateTransition {
                from: self.state.name(),
                action: "select_choice".to_string(),
            });
        }

        // Update the state to ChoiceSelected
        self.state = DialogueState::ChoiceSelected(choice_index);

        Ok(())
    }

    /// Gets the current node from the dialogue asset.
    ///
    /// # Parameters
    ///
    /// * `dialogue` - The dialogue asset being processed
    ///
    /// # Returns
    ///
    /// An optional reference to the current node, or None if there is no current node
    /// or it doesn't exist in the dialogue
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// # use bevy::prelude::*;
    /// # use funkus_dialogue_core::{DialogueAsset, DialogueNode, DialogueRunner};
    /// #
    /// fn process_current_node(
    ///     dialogue_assets: Res<Assets<DialogueAsset>>,
    ///     dialogue_query: Query<&DialogueRunner>,
    /// ) {
    ///     for runner in dialogue_query.iter() {
    ///         if let Some(dialogue) = dialogue_assets.get(&runner.dialogue_handle) {
    ///             if let Some(node) = runner.current_node(dialogue) {
    ///                 match node {
    ///                     DialogueNode::Text { text, .. } => {
    ///                         println!("Current text: {}", text);
    ///                     },
    ///                     DialogueNode::Choice { prompt, .. } => {
    ///                         println!("Current prompt: {:?}", prompt);
    ///                     }
    ///                 }
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    pub fn current_node<'a>(&self, dialogue: &'a DialogueAsset) -> Option<&'a DialogueNode> {
        self.current_node_id
            .and_then(|id| dialogue.graph.get_node(id))
    }

    /// Checks if the dialogue has finished.
    ///
    /// # Returns
    ///
    /// `true` if the dialogue is in the Finished state, `false` otherwise
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// # use bevy::prelude::*;
    /// # use funkus_dialogue_core::DialogueRunner;
    /// #
    /// fn check_dialogue_status(dialogue_query: Query<&DialogueRunner>) {
    ///     for runner in dialogue_query.iter() {
    ///         if runner.is_finished() {
    ///             println!("Dialogue has ended!");
    ///         }
    ///     }
    /// }
    /// ```
    pub fn is_finished(&self) -> bool {
        self.state == DialogueState::Finished
    }

    /// Stops the dialogue and returns to inactive state.
    ///
    /// This method resets the dialogue runner to its initial state,
    /// clearing the current node and any selected choices.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// # use bevy::prelude::*;
    /// # use funkus_dialogue_core::DialogueRunner;
    /// #
    /// fn stop_dialogues(mut dialogue_query: Query<&mut DialogueRunner>) {
    ///     for mut runner in dialogue_query.iter_mut() {
    ///         runner.stop();
    ///     }
    /// }
    /// ```
    pub fn stop(&mut self) {
        self.state = DialogueState::Inactive;
        self.current_node_id = None;
    }
}
