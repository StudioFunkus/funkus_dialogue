//! Dialogue message registry and runtime dispatch for message nodes.
//!
//! Games define normal Bevy messages and tag them with `#[derive(DialogueMessage)]`.
//! The dialogue plugin then auto-registers metadata for editor/runtime usage and
//! can dispatch typed messages from data-authored message nodes.

use std::any::TypeId;
use std::collections::HashMap;

use bevy::ecs::message::Messages;
use bevy::prelude::*;
use bevy::reflect::{
    ArrayInfo, DynamicArray, DynamicEnum, DynamicList, DynamicStruct, FromReflect, ListInfo,
    TypeInfo,
};
use serde::{Deserialize, Serialize};
use tracing::warn;

use super::{DialogueFieldKind, DialogueValue, detect_field_kind};

type DialogueMessageDispatchFn =
    fn(&mut World, &DialogueMessageCall) -> Result<(), DialogueMessageError>;

/// Marker trait for Bevy messages that should be dispatchable from dialogue nodes.
///
/// Derive `DialogueMessage` on your reflected message type to make it available
/// to the dialogue runtime and editor.
pub trait DialogueMessage:
    Message + Reflect + FromReflect + bevy::reflect::Typed + TypePath
{
    /// Override the message key used in dialogue assets.
    ///
    /// By default this is the Rust type path. Override with
    /// `#[dialogue(key = "...")]` for stable, short keys.
    fn message_key() -> &'static str {
        Self::type_path()
    }
}

/// Type data used to mark reflected message types for dialogue registration.
#[derive(Clone)]
pub struct DialogueMessageTypeData {
    message_key: &'static str,
    dispatch: DialogueMessageDispatchFn,
}

impl DialogueMessageTypeData {
    #[must_use]
    pub const fn new(message_key: &'static str, dispatch: DialogueMessageDispatchFn) -> Self {
        Self {
            message_key,
            dispatch,
        }
    }

    #[must_use]
    pub const fn message_key(&self) -> &'static str {
        self.message_key
    }

    #[must_use]
    pub const fn dispatch(&self) -> DialogueMessageDispatchFn {
        self.dispatch
    }
}

impl<T: DialogueMessage> bevy::reflect::FromType<T> for DialogueMessageTypeData {
    fn from_type() -> Self {
        Self::new(T::message_key(), dispatch_dialogue_message::<T>)
    }
}

#[doc(hidden)]
pub struct DialogueMessageRegistration {
    register: fn(&mut bevy::reflect::TypeRegistry),
    add_message: fn(&mut App),
}

impl DialogueMessageRegistration {
    #[doc(hidden)]
    #[must_use]
    pub const fn new(
        register: fn(&mut bevy::reflect::TypeRegistry),
        add_message: fn(&mut App),
    ) -> Self {
        Self {
            register,
            add_message,
        }
    }
}

inventory::collect!(DialogueMessageRegistration);

/// Field metadata for a dialogue-dispatchable message.
#[derive(Debug, Clone)]
pub struct DialogueMessageField {
    /// Field name on the message struct.
    pub name: String,
    /// Serializable field kind.
    pub kind: DialogueFieldKind,
}

/// Metadata for a dialogue-dispatchable message type.
#[derive(Clone)]
pub struct DialogueMessageDefinition {
    /// Stable key used in dialogue assets.
    pub key: String,
    /// Human-friendly label (type path).
    pub label: String,
    /// Message type backing this definition.
    pub type_id: TypeId,
    /// Declared message fields.
    pub fields: Vec<DialogueMessageField>,
    dispatch: DialogueMessageDispatchFn,
}

impl DialogueMessageDefinition {
    #[must_use]
    const fn dispatch(&self) -> DialogueMessageDispatchFn {
        self.dispatch
    }
}

/// Serializable payload for a dialogue message node.
#[derive(Debug, Clone, Default, PartialEq, Reflect, Serialize, Deserialize)]
#[serde(crate = "serde")]
pub struct DialogueMessageCall {
    /// Message key to dispatch.
    pub key: String,
    /// Named parameters forwarded to the message fields.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<DialogueMessageParam>,
}

impl DialogueMessageCall {
    /// Creates a call targeting the supplied message key.
    #[must_use]
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            params: Vec::new(),
        }
    }

    /// Sets or inserts a named parameter.
    pub fn set_param(&mut self, name: impl Into<String>, value: DialogueValue) {
        let name = name.into();
        if let Some(existing) = self.params.iter_mut().find(|param| param.name == name) {
            existing.value = value;
            return;
        }
        self.params.push(DialogueMessageParam { name, value });
    }

    /// Builder-style helper for inserting one parameter.
    #[must_use]
    pub fn with_param(mut self, name: impl Into<String>, value: DialogueValue) -> Self {
        self.set_param(name, value);
        self
    }

    /// Retrieves a parameter by name.
    #[must_use]
    pub fn param(&self, name: &str) -> Option<&DialogueValue> {
        self.params
            .iter()
            .find(|param| param.name == name)
            .map(|param| &param.value)
    }
}

/// One named parameter in a dialogue message call.
#[derive(Debug, Clone, PartialEq, Reflect, Serialize, Deserialize)]
#[serde(crate = "serde")]
pub struct DialogueMessageParam {
    /// Field name expected by the message type.
    pub name: String,
    /// Value to assign.
    pub value: DialogueValue,
}

/// Registry of dialogue-dispatchable message types.
#[derive(Resource, Default)]
pub struct DialogueMessageRegistry {
    messages: HashMap<String, DialogueMessageDefinition>,
}

impl DialogueMessageRegistry {
    /// Iterates all message definitions.
    pub fn messages(&self) -> impl Iterator<Item = &DialogueMessageDefinition> {
        self.messages.values()
    }

    /// Gets one message definition by key.
    #[must_use]
    pub fn message(&self, key: &str) -> Option<&DialogueMessageDefinition> {
        self.messages.get(key)
    }

    /// Registers message metadata from reflection.
    pub fn register_reflected_message(
        &mut self,
        type_info: &'static TypeInfo,
        message_key: &str,
        dispatch: DialogueMessageDispatchFn,
    ) {
        let Some(struct_info) = type_info.as_struct().ok() else {
            warn!(
                "DialogueMessageRegistry: {} is not a struct message, skipping",
                type_info.type_path()
            );
            return;
        };

        if self.messages.contains_key(message_key) {
            warn!("DialogueMessageRegistry: duplicate message key {message_key} ignored");
            return;
        }

        let mut fields = Vec::new();
        let mut failed = false;
        for field in struct_info.iter() {
            let Some(field_info) = field.type_info() else {
                warn!(
                    "DialogueMessageRegistry: field {}.{} lacks type info",
                    message_key,
                    field.name()
                );
                failed = true;
                break;
            };
            let Some(kind) = detect_field_kind(field_info) else {
                warn!(
                    "DialogueMessageRegistry: field {}.{} has unsupported type {}",
                    message_key,
                    field.name(),
                    field_info.type_path()
                );
                failed = true;
                break;
            };

            fields.push(DialogueMessageField {
                name: field.name().to_string(),
                kind,
            });
        }

        if failed {
            warn!(
                "DialogueMessageRegistry: skipping message {} because at least one field is unsupported",
                message_key
            );
            return;
        }

        self.messages.insert(
            message_key.to_string(),
            DialogueMessageDefinition {
                key: message_key.to_string(),
                label: type_info.type_path().to_string(),
                type_id: type_info.type_id(),
                fields,
                dispatch,
            },
        );
    }

    /// Dispatches one message call into Bevy's `Messages<T>` resource.
    pub fn dispatch(
        &self,
        world: &mut World,
        call: &DialogueMessageCall,
    ) -> Result<(), DialogueMessageError> {
        let definition = self
            .message(&call.key)
            .ok_or_else(|| DialogueMessageError::UnknownMessage(call.key.clone()))?;
        (definition.dispatch())(world, call)
    }

    /// Looks up the dispatch function for a key.
    #[must_use]
    pub fn dispatch_fn(&self, key: &str) -> Option<DialogueMessageDispatchFn> {
        self.message(key).map(DialogueMessageDefinition::dispatch)
    }
}

/// Errors from dialogue message dispatch.
#[derive(Debug, Clone)]
pub enum DialogueMessageError {
    UnknownMessage(String),
    UnsupportedMessageType(String),
    MissingField {
        message_key: String,
        field: String,
    },
    InvalidValueType {
        message_key: String,
        field: String,
        expected: String,
    },
    UnsupportedFieldType {
        message_key: String,
        field: String,
        type_path: String,
    },
    MessageResourceMissing(String),
    BuildFailed(String),
}

impl std::fmt::Display for DialogueMessageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DialogueMessageError::UnknownMessage(key) => {
                write!(f, "Unknown dialogue message key {key}")
            }
            DialogueMessageError::UnsupportedMessageType(name) => {
                write!(f, "Dialogue message {name} is not a struct")
            }
            DialogueMessageError::MissingField { message_key, field } => {
                write!(
                    f,
                    "Dialogue message {message_key} is missing required field {field}"
                )
            }
            DialogueMessageError::InvalidValueType {
                message_key,
                field,
                expected,
            } => {
                write!(
                    f,
                    "Dialogue message {message_key}.{field} has invalid value type; expected {expected}"
                )
            }
            DialogueMessageError::UnsupportedFieldType {
                message_key,
                field,
                type_path,
            } => {
                write!(
                    f,
                    "Dialogue message {message_key}.{field} uses unsupported field type {type_path}"
                )
            }
            DialogueMessageError::MessageResourceMissing(name) => {
                write!(f, "Bevy message resource is missing for {name}")
            }
            DialogueMessageError::BuildFailed(name) => {
                write!(f, "Failed to build dialogue message payload for {name}")
            }
        }
    }
}

impl std::error::Error for DialogueMessageError {}

/// Plugin that auto-registers derived dialogue messages and builds metadata.
#[derive(Default)]
pub struct DialogueMessageRegistryPlugin;

impl Plugin for DialogueMessageRegistryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DialogueMessageRegistry>();

        // Ensure all derived message resources exist as normal Bevy messages.
        for registration in inventory::iter::<DialogueMessageRegistration> {
            (registration.add_message)(app);
        }

        app.add_systems(
            Startup,
            (
                register_dialogue_messages_from_derive,
                build_message_registry_from_reflection,
            )
                .chain(),
        );
    }
}

fn register_dialogue_messages_from_derive(app_registry: ResMut<AppTypeRegistry>) {
    let mut type_registry = app_registry.write();
    for registration in inventory::iter::<DialogueMessageRegistration> {
        (registration.register)(&mut type_registry);
    }
}

fn build_message_registry_from_reflection(
    mut registry: ResMut<DialogueMessageRegistry>,
    app_registry: Res<AppTypeRegistry>,
) {
    let type_registry = app_registry.read();
    for registration in type_registry.iter() {
        let Some(marker) = registration.data::<DialogueMessageTypeData>() else {
            continue;
        };
        registry.register_reflected_message(
            registration.type_info(),
            marker.message_key(),
            marker.dispatch(),
        );
    }
}

fn dispatch_dialogue_message<T: DialogueMessage>(
    world: &mut World,
    call: &DialogueMessageCall,
) -> Result<(), DialogueMessageError> {
    let type_info = <T as bevy::reflect::Typed>::type_info();
    let struct_info = type_info.as_struct().ok().ok_or_else(|| {
        DialogueMessageError::UnsupportedMessageType(type_info.type_path().to_string())
    })?;

    let mut payload = DynamicStruct::default();
    for field in struct_info.iter() {
        let field_name = field.name();
        let value = call
            .param(field_name)
            .ok_or_else(|| DialogueMessageError::MissingField {
                message_key: call.key.clone(),
                field: field_name.to_string(),
            })?;
        let field_info =
            field
                .type_info()
                .ok_or_else(|| DialogueMessageError::UnsupportedFieldType {
                    message_key: call.key.clone(),
                    field: field_name.to_string(),
                    type_path: "<unknown>".to_string(),
                })?;
        let reflect_value = value_to_reflect_value(value, field_info, &call.key, field_name)?;
        payload.insert_boxed(field_name, reflect_value);
    }

    let message = T::from_reflect(&payload)
        .ok_or_else(|| DialogueMessageError::BuildFailed(type_info.type_path().to_string()))?;

    let Some(mut messages) = world.get_resource_mut::<Messages<T>>() else {
        return Err(DialogueMessageError::MessageResourceMissing(
            type_info.type_path().to_string(),
        ));
    };
    messages.write(message);
    Ok(())
}

fn value_to_reflect_value(
    value: &DialogueValue,
    type_info: &'static TypeInfo,
    message_key: &str,
    field_name: &str,
) -> Result<Box<dyn bevy::reflect::PartialReflect>, DialogueMessageError> {
    if type_info.type_id() == TypeId::of::<bool>() {
        let DialogueValue::Bool(v) = value else {
            return Err(invalid_value_type(message_key, field_name, type_info));
        };
        return Ok(Box::new(*v));
    }

    if type_info.type_id() == TypeId::of::<i64>() {
        let DialogueValue::Int(v) = value else {
            return Err(invalid_value_type(message_key, field_name, type_info));
        };
        return Ok(Box::new(*v));
    }

    if type_info.type_id() == TypeId::of::<i32>() {
        let DialogueValue::Int(v) = value else {
            return Err(invalid_value_type(message_key, field_name, type_info));
        };
        return Ok(Box::new(*v as i32));
    }

    if type_info.type_id() == TypeId::of::<u32>() {
        let DialogueValue::Int(v) = value else {
            return Err(invalid_value_type(message_key, field_name, type_info));
        };
        if *v < 0 {
            return Err(invalid_value_type(message_key, field_name, type_info));
        }
        return Ok(Box::new(*v as u32));
    }

    if type_info.type_id() == TypeId::of::<f64>() {
        let DialogueValue::Float(v) = value else {
            return Err(invalid_value_type(message_key, field_name, type_info));
        };
        return Ok(Box::new(*v));
    }

    if type_info.type_id() == TypeId::of::<f32>() {
        let DialogueValue::Float(v) = value else {
            return Err(invalid_value_type(message_key, field_name, type_info));
        };
        return Ok(Box::new(*v as f32));
    }

    if type_info.type_id() == TypeId::of::<String>() {
        let DialogueValue::String(v) = value else {
            return Err(invalid_value_type(message_key, field_name, type_info));
        };
        return Ok(Box::new(v.clone()));
    }

    match type_info {
        TypeInfo::Enum(enum_info) => {
            enum_value_to_reflect(value, enum_info, message_key, field_name)
        }
        TypeInfo::List(list_info) => {
            list_value_to_reflect(value, list_info, message_key, field_name)
        }
        TypeInfo::Array(array_info) => {
            array_value_to_reflect(value, array_info, message_key, field_name)
        }
        _ => Err(DialogueMessageError::UnsupportedFieldType {
            message_key: message_key.to_string(),
            field: field_name.to_string(),
            type_path: type_info.type_path().to_string(),
        }),
    }
}

fn enum_value_to_reflect(
    value: &DialogueValue,
    enum_info: &bevy::reflect::EnumInfo,
    message_key: &str,
    field_name: &str,
) -> Result<Box<dyn bevy::reflect::PartialReflect>, DialogueMessageError> {
    let DialogueValue::Enum(variant_name) = value else {
        return Err(DialogueMessageError::InvalidValueType {
            message_key: message_key.to_string(),
            field: field_name.to_string(),
            expected: "enum".to_string(),
        });
    };

    let mut found_index = None;
    for (index, name) in enum_info.variant_names().iter().enumerate() {
        if *name == variant_name {
            found_index = Some(index);
            break;
        }
    }
    let Some(index) = found_index else {
        return Err(DialogueMessageError::InvalidValueType {
            message_key: message_key.to_string(),
            field: field_name.to_string(),
            expected: "valid enum variant name".to_string(),
        });
    };

    let Some(variant) = enum_info.variant(variant_name) else {
        return Err(DialogueMessageError::InvalidValueType {
            message_key: message_key.to_string(),
            field: field_name.to_string(),
            expected: "valid enum variant".to_string(),
        });
    };
    if variant.variant_type() != bevy::reflect::VariantType::Unit {
        return Err(DialogueMessageError::UnsupportedFieldType {
            message_key: message_key.to_string(),
            field: field_name.to_string(),
            type_path: "only unit enums are supported".to_string(),
        });
    }

    Ok(Box::new(DynamicEnum::new_with_index(
        index,
        variant_name.clone(),
        bevy::reflect::DynamicVariant::Unit,
    )))
}

fn list_value_to_reflect(
    value: &DialogueValue,
    list_info: &ListInfo,
    message_key: &str,
    field_name: &str,
) -> Result<Box<dyn bevy::reflect::PartialReflect>, DialogueMessageError> {
    let DialogueValue::List(values) = value else {
        return Err(DialogueMessageError::InvalidValueType {
            message_key: message_key.to_string(),
            field: field_name.to_string(),
            expected: "list".to_string(),
        });
    };

    let item_info =
        list_info
            .item_info()
            .ok_or_else(|| DialogueMessageError::UnsupportedFieldType {
                message_key: message_key.to_string(),
                field: field_name.to_string(),
                type_path: "list without item type info".to_string(),
            })?;

    let mut list = DynamicList::default();
    for item in values {
        let reflect = value_to_reflect_value(item, item_info, message_key, field_name)?;
        list.push_box(reflect);
    }
    Ok(Box::new(list))
}

fn array_value_to_reflect(
    value: &DialogueValue,
    array_info: &ArrayInfo,
    message_key: &str,
    field_name: &str,
) -> Result<Box<dyn bevy::reflect::PartialReflect>, DialogueMessageError> {
    let DialogueValue::List(values) = value else {
        return Err(DialogueMessageError::InvalidValueType {
            message_key: message_key.to_string(),
            field: field_name.to_string(),
            expected: "array/list".to_string(),
        });
    };

    let item_info =
        array_info
            .item_info()
            .ok_or_else(|| DialogueMessageError::UnsupportedFieldType {
                message_key: message_key.to_string(),
                field: field_name.to_string(),
                type_path: "array without item type info".to_string(),
            })?;

    if values.len() != array_info.capacity() {
        return Err(DialogueMessageError::InvalidValueType {
            message_key: message_key.to_string(),
            field: field_name.to_string(),
            expected: format!("array length {}", array_info.capacity()),
        });
    }

    let mut boxed: Vec<Box<dyn bevy::reflect::PartialReflect>> = Vec::with_capacity(values.len());
    for item in values {
        boxed.push(value_to_reflect_value(
            item,
            item_info,
            message_key,
            field_name,
        )?);
    }

    Ok(Box::new(DynamicArray::new(boxed.into_boxed_slice())))
}

fn invalid_value_type(
    message_key: &str,
    field_name: &str,
    type_info: &'static TypeInfo,
) -> DialogueMessageError {
    let expected = detect_field_kind(type_info)
        .map(|kind| format!("{kind:?}"))
        .unwrap_or_else(|| type_info.type_path().to_string());
    DialogueMessageError::InvalidValueType {
        message_key: message_key.to_string(),
        field: field_name.to_string(),
        expected,
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::message::{MessageCursor, Messages};
    use bevy::prelude::*;

    use super::{DialogueMessageCall, DialogueMessageRegistry, DialogueMessageRegistryPlugin};
    use crate::registry::DialogueValue;

    #[derive(Debug, Clone, PartialEq, Eq, Reflect)]
    enum AutoMessageMood {
        Neutral,
        Happy,
    }

    #[derive(Message, Clone, Debug, PartialEq, Reflect, crate::DialogueMessage)]
    #[dialogue(key = "auto_message")]
    struct AutoMessage {
        amount: i32,
        speaker: String,
        mood: AutoMessageMood,
    }

    #[test]
    fn plugin_auto_registers_derived_dialogue_messages() {
        let mut app = App::new();
        app.add_plugins(DialogueMessageRegistryPlugin);
        app.update();

        let registry = app.world().resource::<DialogueMessageRegistry>();
        let definition = registry
            .message("auto_message")
            .expect("derived message should be registered");
        assert_eq!(definition.fields.len(), 3);
        assert!(app.world().contains_resource::<Messages<AutoMessage>>());
    }

    #[test]
    fn registry_dispatches_typed_bevy_messages() {
        let mut app = App::new();
        app.add_plugins(DialogueMessageRegistryPlugin);
        app.update();

        let call = DialogueMessageCall::new("auto_message")
            .with_param("amount", DialogueValue::Int(7))
            .with_param("speaker", DialogueValue::String("Guide".to_string()))
            .with_param("mood", DialogueValue::Enum("Happy".to_string()));

        let dispatch = app
            .world()
            .resource::<DialogueMessageRegistry>()
            .dispatch_fn("auto_message")
            .expect("dispatch function exists");
        dispatch(app.world_mut(), &call).expect("dispatch succeeds");

        let mut cursor = MessageCursor::<AutoMessage>::default();
        let sent: Vec<AutoMessage> = {
            let messages = app.world().resource::<Messages<AutoMessage>>();
            cursor.read(messages).cloned().collect()
        };
        assert_eq!(sent.len(), 1);
        assert_eq!(
            sent[0],
            AutoMessage {
                amount: 7,
                speaker: "Guide".to_string(),
                mood: AutoMessageMood::Happy,
            }
        );
    }
}
