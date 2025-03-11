//! # Error types for the dialogue system.
//! 
//! This module defines the error types and results used throughout the dialogue system.
//! It provides structured error handling for dialogue operations, making it easier to
//! identify and handle specific error conditions.

use thiserror::Error;
use bevy::prelude::*;
use crate::graph::NodeId;

/// Errors that can occur in the dialogue system.
/// 
/// This enum represents all the possible errors that can occur during
/// dialogue operations. Each variant includes context information to
/// help diagnose the issue.
/// 
/// # Examples
/// 
/// ```rust
/// use funkus_dialogue::{DialogueRunner, DialogueAsset, graph::NodeId};
/// use bevy::prelude::*;
/// 
/// fn handle_dialogue_errors(
///     dialogue_assets: Res<Assets<DialogueAsset>>,
///     mut query: Query<&mut DialogueRunner>,
/// ) {
///     for mut runner in query.iter_mut() {
///         if let Some(dialogue) = dialogue_assets.get(&runner.dialogue_handle) {
///             match runner.advance(dialogue) {
///                 Ok(()) => println!("Dialogue advanced successfully"),
///                 Err(err) => match err {
///                     funkus_dialogue::error::DialogueError::NoCurrentNode => {
///                         println!("No current node in dialogue")
///                     },
///                     funkus_dialogue::error::DialogueError::NodeNotFound(id) => {
///                         println!("Node {:?} not found", id)
///                     },
///                     // Handle other error types
///                     _ => println!("Other error: {}", err),
///                 },
///             }
///         }
///     }
/// }
/// ```
#[derive(Error, Debug, Clone)]
pub enum DialogueError {
    /// No current node is active
    #[error("No current dialogue node")]
    NoCurrentNode,
    
    /// Node not found in the dialogue graph
    #[error("Node {0:?} not found in dialogue")]
    NodeNotFound(NodeId),
    
    /// Next node not found
    #[error("Next node {0:?} not found")]
    NextNodeNotFound(NodeId),
    
    /// No choice selected for a choice node
    #[error("No choice selected for choice node")]
    NoChoiceSelected,
    
    /// Selected choice index is out of bounds
    #[error("Invalid choice index: {0} (max: {1})")]
    InvalidChoiceIndex(usize, usize),
    
    /// Invalid state transition
    #[error("Invalid state transition: from {from:?} with action {action}")]
    InvalidStateTransition {
        from: String,
        action: String,
    },
    
    /// General graph error
    #[error("Graph error: {0}")]
    GraphError(String),
    
    /// Asset not loaded
    #[error("Dialogue asset not loaded")]
    AssetNotLoaded,
}

/// Result type for dialogue operations
/// 
/// This is a convenience type alias for Result with DialogueError as the error type.
/// It's used throughout the dialogue system for operations that can fail.
/// 
/// # Examples
/// 
/// ```rust
/// use funkus_dialogue::error::{DialogueResult, DialogueError};
/// 
/// fn some_dialogue_operation() -> DialogueResult<String> {
///     // An operation that could fail
///     if condition_is_met() {
///         Ok("Operation succeeded".to_string())
///     } else {
///         Err(DialogueError::GraphError("Something went wrong".to_string()))
///     }
/// }
/// ```
pub type DialogueResult<T> = Result<T, DialogueError>;
