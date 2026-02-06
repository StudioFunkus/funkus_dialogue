//! Value types for dialogue-driven resource changes.
//!
//! These are intentionally conservative and serializable. They are used both
//! in dialogue assets and in the editor UI.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Operations that can be applied to a registered dialogue field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect, Serialize, Deserialize)]
#[serde(crate = "serde", rename_all = "snake_case")]
pub enum DialogueOperation {
    /// Replace the field's value with the provided value.
    Set,
    /// Add the provided value to the field (numeric only).
    Add,
    /// Subtract the provided value from the field (numeric only).
    Subtract,
    /// Toggle the field's boolean value.
    Toggle,
    /// Append a value to a list field.
    Push,
    /// Remove the first matching value from a list field.
    Remove,
    /// Clear a list field.
    Clear,
}

/// Serializable values that can be applied to dialogue fields.
#[derive(Debug, Clone, PartialEq, Reflect, Serialize, Deserialize)]
#[serde(
    crate = "serde",
    rename_all = "snake_case",
    tag = "type",
    content = "value"
)]
pub enum DialogueValue {
    /// A boolean value.
    Bool(bool),
    /// A signed integer value.
    Int(i64),
    /// A floating point value.
    Float(f64),
    /// A string value.
    String(String),
    /// A unit enum variant by name.
    Enum(String),
    /// A list of values (for Vec/array fields).
    List(Vec<DialogueValue>),
    /// Absence of a value (used for optional fields).
    None,
}

impl DialogueValue {
    /// Returns the value's primitive kind (if any).
    #[must_use]
    pub const fn kind(&self) -> DialogueValueKind {
        match self {
            DialogueValue::Bool(_) => DialogueValueKind::Bool,
            DialogueValue::Int(_) => DialogueValueKind::Int,
            DialogueValue::Float(_) => DialogueValueKind::Float,
            DialogueValue::String(_) => DialogueValueKind::String,
            DialogueValue::Enum(_) => DialogueValueKind::Enum,
            DialogueValue::List(_) => DialogueValueKind::List,
            DialogueValue::None => DialogueValueKind::None,
        }
    }
}

/// Lightweight descriptor for the kind of a [`DialogueValue`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogueValueKind {
    Bool,
    Int,
    Float,
    String,
    Enum,
    List,
    None,
}

/// A single resource update performed by a dialogue node.
#[derive(Debug, Clone, PartialEq, Reflect, Serialize, Deserialize)]
#[serde(crate = "serde")]
pub struct DialogueEffect {
    /// The registry key for the field to update.
    pub key: String,
    /// The operation to apply.
    pub op: DialogueOperation,
    /// The value to use for the operation.
    pub value: DialogueValue,
}

impl DialogueEffect {
    /// Convenience constructor for a simple set operation.
    #[must_use]
    pub fn set(key: impl Into<String>, value: DialogueValue) -> Self {
        Self {
            key: key.into(),
            op: DialogueOperation::Set,
            value,
        }
    }
}
