//! Dialogue resource registry and reflection-backed field access.
//!
//! This module provides a data-driven registry for game resources that can be
//! read or mutated by dialogue nodes. The registry is built using Bevy's
//! reflection metadata to ensure compatibility with serialization and tooling.

use std::any::TypeId;
use std::collections::HashMap;

use bevy::ecs::change_detection::MutUntyped;
use bevy::prelude::*;
use bevy::reflect::{ArrayInfo, ListInfo, ReflectFromPtr, TypeInfo};
use serde::{Deserialize, Serialize};
use tracing::warn;

mod value;

pub use value::{DialogueEffect, DialogueOperation, DialogueValue, DialogueValueKind};

/// Marker trait for resources that should be included in the dialogue registry.
///
/// Register types with [`register_dialogue_resource`](DialogueRegistryAppExt::register_dialogue_resource)
/// to make them available at runtime and in the editor.
pub trait DialogueResource: Resource + Reflect + TypePath {
    /// Override the registry prefix for this resource.
    ///
    /// By default, this is the Rust type path, but games can return a shorter,
    /// stable prefix (for example `"game"` or `"npc_state"`).
    fn resource_key() -> &'static str {
        Self::type_path()
    }
}

/// Type data used to mark reflected resources for registration.
#[derive(Clone)]
pub struct DialogueResourceTypeData {
    resource_key: &'static str,
}

impl DialogueResourceTypeData {
    #[must_use]
    pub const fn new(resource_key: &'static str) -> Self {
        Self { resource_key }
    }

    #[must_use]
    pub const fn resource_key(&self) -> &'static str {
        self.resource_key
    }
}

impl<T: DialogueResource> bevy::reflect::FromType<T> for DialogueResourceTypeData {
    fn from_type() -> Self {
        Self::new(T::resource_key())
    }
}

/// Describes the expected type of a registry field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(crate = "serde", rename_all = "snake_case")]
pub enum DialogueFieldKind {
    Bool,
    Int,
    Float,
    String,
    Enum {
        variants: Vec<String>,
    },
    List(Box<DialogueFieldKind>),
    Array {
        len: usize,
        element: Box<DialogueFieldKind>,
    },
}

impl DialogueFieldKind {
    /// Returns the allowed operations for this field type.
    #[must_use]
    pub fn allowed_operations(&self) -> &'static [DialogueOperation] {
        match self {
            DialogueFieldKind::Bool => &[DialogueOperation::Set, DialogueOperation::Toggle],
            DialogueFieldKind::Int | DialogueFieldKind::Float => &[
                DialogueOperation::Set,
                DialogueOperation::Add,
                DialogueOperation::Subtract,
            ],
            DialogueFieldKind::Enum { .. } => &[DialogueOperation::Set],
            DialogueFieldKind::String => &[DialogueOperation::Set],
            DialogueFieldKind::List(_) => &[
                DialogueOperation::Set,
                DialogueOperation::Push,
                DialogueOperation::Remove,
                DialogueOperation::Clear,
            ],
            DialogueFieldKind::Array { .. } => &[DialogueOperation::Set],
        }
    }
}

/// A single field exposed through the dialogue registry.
#[derive(Debug, Clone)]
pub struct DialogueField {
    /// Fully qualified key used in dialogue assets.
    pub key: String,
    /// Short label used for UI.
    pub label: String,
    /// The field's type.
    pub kind: DialogueFieldKind,
    /// Resource type backing this field.
    pub resource_type_id: TypeId,
    /// Field name within the resource.
    pub field_name: &'static str,
}

/// Registry holding all fields available to the dialogue system.
#[derive(Resource, Default)]
pub struct DialogueRegistry {
    fields: HashMap<String, DialogueField>,
}

impl DialogueRegistry {
    /// Returns an iterator over all registered fields.
    pub fn fields(&self) -> impl Iterator<Item = &DialogueField> {
        self.fields.values()
    }

    /// Returns a field by its key.
    #[must_use]
    pub fn field(&self, key: &str) -> Option<&DialogueField> {
        self.fields.get(key)
    }

    /// Registers a resource type from reflection metadata.
    pub fn register_reflected_resource(
        &mut self,
        type_info: &'static TypeInfo,
        resource_key: &str,
    ) {
        let Some(struct_info) = type_info.as_struct().ok() else {
            warn!(
                "DialogueRegistry: {} is not a struct, skipping",
                type_info.type_path()
            );
            return;
        };

        for field in struct_info.iter() {
            let Some(field_info) = field.type_info() else {
                warn!(
                    "DialogueRegistry: field {}.{} lacks type info",
                    resource_key,
                    field.name()
                );
                continue;
            };

            let Some(kind) = detect_field_kind(field_info) else {
                warn!(
                    "DialogueRegistry: field {}.{} has unsupported type {}",
                    resource_key,
                    field.name(),
                    field_info.type_path()
                );
                continue;
            };

            let key = format!("{resource_key}.{}", field.name());
            let label = field.name().to_string();
            if self.fields.contains_key(&key) {
                warn!("DialogueRegistry: duplicate key {key} ignored");
                continue;
            }
            self.fields.insert(
                key.clone(),
                DialogueField {
                    key,
                    label,
                    kind,
                    resource_type_id: type_info.type_id(),
                    field_name: field.name(),
                },
            );
        }
    }

    /// Reads a field value from the world.
    pub fn read_value(
        &self,
        world: &World,
        type_registry: &bevy::reflect::TypeRegistry,
        key: &str,
    ) -> Result<DialogueValue, DialogueRegistryError> {
        let field = self
            .field(key)
            .ok_or_else(|| DialogueRegistryError::UnknownKey(key.to_string()))?;

        let (resource_ptr, registration) = resolve_resource_ptr(world, type_registry, field)?;
        let reflect_from_ptr = registration
            .data::<ReflectFromPtr>()
            .ok_or(DialogueRegistryError::MissingReflect(field.key.clone()))?;

        let reflect = unsafe { reflect_from_ptr.as_reflect(resource_ptr) };
        let struct_reflect = reflect
            .reflect_ref()
            .as_struct()
            .ok()
            .ok_or_else(|| DialogueRegistryError::UnsupportedResource(field.key.clone()))?;
        let field_reflect = struct_reflect
            .field(field.field_name)
            .ok_or_else(|| DialogueRegistryError::MissingField(field.key.clone()))?;

        reflect_to_value(field_reflect, &field.kind)
            .ok_or_else(|| DialogueRegistryError::TypeMismatch(field.key.clone()))
    }

    /// Applies an effect to the world.
    pub fn apply_effect(
        &self,
        world: &mut World,
        type_registry: &bevy::reflect::TypeRegistry,
        effect: &DialogueEffect,
    ) -> Result<(), DialogueRegistryError> {
        let field = self
            .field(&effect.key)
            .ok_or_else(|| DialogueRegistryError::UnknownKey(effect.key.clone()))?;
        let reflect_from_ptr = resolve_reflect_from_ptr(type_registry, field)?;
        apply_effect_with_field_and_reflect(world, field, &reflect_from_ptr, effect)
    }
}

pub(crate) fn apply_effect_with_field_and_reflect(
    world: &mut World,
    field: &DialogueField,
    reflect_from_ptr: &ReflectFromPtr,
    effect: &DialogueEffect,
) -> Result<(), DialogueRegistryError> {
    let resource_ptr = resolve_resource_mut_ptr(world, field)?;

    let reflect = unsafe { reflect_from_ptr.as_reflect_mut(resource_ptr.into_inner()) };
    let mut_struct = reflect
        .reflect_mut()
        .as_struct()
        .ok()
        .ok_or_else(|| DialogueRegistryError::UnsupportedResource(field.key.clone()))?;
    let field_reflect = mut_struct
        .field_mut(field.field_name)
        .ok_or_else(|| DialogueRegistryError::MissingField(field.key.clone()))?;

    apply_value(field_reflect, &field.kind, &effect.op, &effect.value)
}

/// Errors returned by dialogue registry operations.
#[derive(Debug, Clone)]
pub enum DialogueRegistryError {
    UnknownKey(String),
    MissingReflect(String),
    UnsupportedResource(String),
    MissingField(String),
    TypeMismatch(String),
    InvalidOperation(String),
}

impl std::fmt::Display for DialogueRegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DialogueRegistryError::UnknownKey(key) => write!(f, "Unknown registry key {key}"),
            DialogueRegistryError::MissingReflect(key) => {
                write!(f, "Missing ReflectFromPtr for {key}")
            }
            DialogueRegistryError::UnsupportedResource(key) => {
                write!(f, "Resource for {key} is not a struct")
            }
            DialogueRegistryError::MissingField(key) => write!(f, "Missing field for {key}"),
            DialogueRegistryError::TypeMismatch(key) => write!(f, "Type mismatch for {key}"),
            DialogueRegistryError::InvalidOperation(key) => {
                write!(f, "Invalid operation for {key}")
            }
        }
    }
}

impl std::error::Error for DialogueRegistryError {}

/// Plugin that builds the registry from reflected resource types.
#[derive(Default)]
pub struct DialogueRegistryPlugin;

impl Plugin for DialogueRegistryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DialogueRegistry>()
            .add_systems(Startup, build_registry_from_reflection);
    }
}

fn build_registry_from_reflection(
    mut registry: ResMut<DialogueRegistry>,
    app_registry: Res<AppTypeRegistry>,
) {
    let type_registry = app_registry.read();
    for registration in type_registry.iter() {
        let Some(marker) = registration.data::<DialogueResourceTypeData>() else {
            continue;
        };
        registry.register_reflected_resource(registration.type_info(), marker.resource_key());
    }
}

/// App extension for registering dialogue resources.
pub trait DialogueRegistryAppExt {
    /// Registers a resource type for dialogue reflection.
    fn register_dialogue_resource<T: DialogueResource + bevy::reflect::GetTypeRegistration>(
        &mut self,
    ) -> &mut Self;
}

impl DialogueRegistryAppExt for App {
    fn register_dialogue_resource<T: DialogueResource + bevy::reflect::GetTypeRegistration>(
        &mut self,
    ) -> &mut Self {
        self.register_type::<T>()
            .register_type_data::<T, DialogueResourceTypeData>();
        self
    }
}

fn detect_field_kind(type_info: &'static TypeInfo) -> Option<DialogueFieldKind> {
    if type_info.type_id() == TypeId::of::<bool>() {
        return Some(DialogueFieldKind::Bool);
    }
    if type_info.type_id() == TypeId::of::<i64>() {
        return Some(DialogueFieldKind::Int);
    }
    if type_info.type_id() == TypeId::of::<i32>() {
        return Some(DialogueFieldKind::Int);
    }
    if type_info.type_id() == TypeId::of::<u32>() {
        return Some(DialogueFieldKind::Int);
    }
    if type_info.type_id() == TypeId::of::<f32>() {
        return Some(DialogueFieldKind::Float);
    }
    if type_info.type_id() == TypeId::of::<f64>() {
        return Some(DialogueFieldKind::Float);
    }
    if type_info.type_id() == TypeId::of::<String>() {
        return Some(DialogueFieldKind::String);
    }

    match type_info {
        TypeInfo::Enum(enum_info) => detect_enum_kind(enum_info),
        TypeInfo::List(list_info) => {
            detect_list_kind(list_info).map(|inner| DialogueFieldKind::List(Box::new(inner)))
        }
        TypeInfo::Array(array_info) => detect_array_kind(array_info),
        _ => None,
    }
}

fn detect_list_kind(list_info: &ListInfo) -> Option<DialogueFieldKind> {
    list_info
        .item_info()
        .and_then(|info| detect_field_kind(info))
}

fn detect_array_kind(array_info: &ArrayInfo) -> Option<DialogueFieldKind> {
    let element = array_info
        .item_info()
        .and_then(|info| detect_field_kind(info))?;
    Some(DialogueFieldKind::Array {
        len: array_info.capacity(),
        element: Box::new(element),
    })
}

fn detect_enum_kind(enum_info: &bevy::reflect::EnumInfo) -> Option<DialogueFieldKind> {
    let mut variants = Vec::new();
    for name in enum_info.variant_names() {
        let Some(variant) = enum_info.variant(name) else {
            return None;
        };
        if variant.variant_type() != bevy::reflect::VariantType::Unit {
            return None;
        }
        variants.push((*name).to_string());
    }
    Some(DialogueFieldKind::Enum { variants })
}

fn resolve_resource_ptr<'a>(
    world: &'a World,
    type_registry: &'a bevy::reflect::TypeRegistry,
    field: &DialogueField,
) -> Result<(bevy::ptr::Ptr<'a>, &'a bevy::reflect::TypeRegistration), DialogueRegistryError> {
    let registration = type_registry
        .get(field.resource_type_id)
        .ok_or_else(|| DialogueRegistryError::MissingReflect(field.key.clone()))?;
    let component_id = world
        .components()
        .get_resource_id(field.resource_type_id)
        .ok_or_else(|| DialogueRegistryError::MissingField(field.key.clone()))?;
    let ptr = world
        .get_resource_by_id(component_id)
        .ok_or_else(|| DialogueRegistryError::MissingField(field.key.clone()))?;
    Ok((ptr, registration))
}

pub(crate) fn resolve_reflect_from_ptr(
    type_registry: &bevy::reflect::TypeRegistry,
    field: &DialogueField,
) -> Result<ReflectFromPtr, DialogueRegistryError> {
    let registration = type_registry
        .get(field.resource_type_id)
        .ok_or_else(|| DialogueRegistryError::MissingReflect(field.key.clone()))?;
    registration
        .data::<ReflectFromPtr>()
        .cloned()
        .ok_or(DialogueRegistryError::MissingReflect(field.key.clone()))
}

fn resolve_resource_mut_ptr<'a>(
    world: &'a mut World,
    field: &DialogueField,
) -> Result<MutUntyped<'a>, DialogueRegistryError> {
    let component_id = world
        .components()
        .get_resource_id(field.resource_type_id)
        .ok_or_else(|| DialogueRegistryError::MissingField(field.key.clone()))?;
    let ptr = world
        .get_resource_mut_by_id(component_id)
        .ok_or_else(|| DialogueRegistryError::MissingField(field.key.clone()))?;
    Ok(ptr)
}

fn reflect_to_value(
    value: &dyn bevy::reflect::PartialReflect,
    kind: &DialogueFieldKind,
) -> Option<DialogueValue> {
    match kind {
        DialogueFieldKind::Bool => value
            .try_downcast_ref::<bool>()
            .map(|v| DialogueValue::Bool(*v)),
        DialogueFieldKind::Int => {
            if let Some(v) = value.try_downcast_ref::<i64>() {
                Some(DialogueValue::Int(*v))
            } else if let Some(v) = value.try_downcast_ref::<i32>() {
                Some(DialogueValue::Int(*v as i64))
            } else if let Some(v) = value.try_downcast_ref::<u32>() {
                Some(DialogueValue::Int(*v as i64))
            } else {
                None
            }
        }
        DialogueFieldKind::Float => {
            if let Some(v) = value.try_downcast_ref::<f32>() {
                Some(DialogueValue::Float(*v as f64))
            } else if let Some(v) = value.try_downcast_ref::<f64>() {
                Some(DialogueValue::Float(*v))
            } else {
                None
            }
        }
        DialogueFieldKind::String => value
            .try_downcast_ref::<String>()
            .map(|v| DialogueValue::String(v.clone())),
        DialogueFieldKind::Enum { .. } => value
            .reflect_ref()
            .as_enum()
            .ok()
            .map(|enum_ref| DialogueValue::Enum(enum_ref.variant_name().to_string())),
        DialogueFieldKind::List(inner) => {
            let list = value.reflect_ref().as_list().ok()?;
            let mut values = Vec::with_capacity(list.len());
            for item in list.iter() {
                values.push(reflect_to_value(item, inner)?);
            }
            Some(DialogueValue::List(values))
        }
        DialogueFieldKind::Array { element, .. } => {
            let array = value.reflect_ref().as_array().ok()?;
            let mut values = Vec::with_capacity(array.len());
            for item in array.iter() {
                values.push(reflect_to_value(item, element)?);
            }
            Some(DialogueValue::List(values))
        }
    }
}

fn apply_value(
    field: &mut dyn bevy::reflect::PartialReflect,
    kind: &DialogueFieldKind,
    op: &DialogueOperation,
    value: &DialogueValue,
) -> Result<(), DialogueRegistryError> {
    match kind {
        DialogueFieldKind::Bool => apply_bool(field, op, value),
        DialogueFieldKind::Int => apply_int(field, op, value),
        DialogueFieldKind::Float => apply_float(field, op, value),
        DialogueFieldKind::String => apply_string(field, op, value),
        DialogueFieldKind::Enum { variants } => apply_enum(field, variants, op, value),
        DialogueFieldKind::List(inner) => apply_list(field, inner, op, value),
        DialogueFieldKind::Array { element, len } => apply_array(field, element, *len, op, value),
    }
}

fn apply_bool(
    field: &mut dyn bevy::reflect::PartialReflect,
    op: &DialogueOperation,
    value: &DialogueValue,
) -> Result<(), DialogueRegistryError> {
    let target = field
        .try_downcast_mut::<bool>()
        .ok_or_else(|| DialogueRegistryError::TypeMismatch("bool".to_string()))?;
    match op {
        DialogueOperation::Set => {
            let DialogueValue::Bool(v) = value else {
                return Err(DialogueRegistryError::TypeMismatch("bool".to_string()));
            };
            *target = *v;
        }
        DialogueOperation::Toggle => {
            *target = !*target;
        }
        _ => return Err(DialogueRegistryError::InvalidOperation("bool".to_string())),
    }
    Ok(())
}

fn apply_int(
    field: &mut dyn bevy::reflect::PartialReflect,
    op: &DialogueOperation,
    value: &DialogueValue,
) -> Result<(), DialogueRegistryError> {
    if let Some(target) = field.try_downcast_mut::<i64>() {
        apply_numeric_int(target, op, value)
    } else if let Some(target) = field.try_downcast_mut::<i32>() {
        let mut temp = *target as i64;
        apply_numeric_int(&mut temp, op, value)?;
        *target = temp as i32;
        Ok(())
    } else if let Some(target) = field.try_downcast_mut::<u32>() {
        let mut temp = *target as i64;
        apply_numeric_int(&mut temp, op, value)?;
        *target = temp.max(0) as u32;
        Ok(())
    } else {
        Err(DialogueRegistryError::TypeMismatch("int".to_string()))
    }
}

fn apply_numeric_int(
    target: &mut i64,
    op: &DialogueOperation,
    value: &DialogueValue,
) -> Result<(), DialogueRegistryError> {
    let DialogueValue::Int(v) = value else {
        return Err(DialogueRegistryError::TypeMismatch("int".to_string()));
    };
    match op {
        DialogueOperation::Set => *target = *v,
        DialogueOperation::Add => *target += *v,
        DialogueOperation::Subtract => *target -= *v,
        _ => return Err(DialogueRegistryError::InvalidOperation("int".to_string())),
    }
    Ok(())
}

fn apply_float(
    field: &mut dyn bevy::reflect::PartialReflect,
    op: &DialogueOperation,
    value: &DialogueValue,
) -> Result<(), DialogueRegistryError> {
    if let Some(target) = field.try_downcast_mut::<f64>() {
        apply_numeric_float(target, op, value)
    } else if let Some(target) = field.try_downcast_mut::<f32>() {
        let mut temp = *target as f64;
        apply_numeric_float(&mut temp, op, value)?;
        *target = temp as f32;
        Ok(())
    } else {
        Err(DialogueRegistryError::TypeMismatch("float".to_string()))
    }
}

fn apply_numeric_float(
    target: &mut f64,
    op: &DialogueOperation,
    value: &DialogueValue,
) -> Result<(), DialogueRegistryError> {
    let DialogueValue::Float(v) = value else {
        return Err(DialogueRegistryError::TypeMismatch("float".to_string()));
    };
    match op {
        DialogueOperation::Set => *target = *v,
        DialogueOperation::Add => *target += *v,
        DialogueOperation::Subtract => *target -= *v,
        _ => return Err(DialogueRegistryError::InvalidOperation("float".to_string())),
    }
    Ok(())
}

fn apply_string(
    field: &mut dyn bevy::reflect::PartialReflect,
    op: &DialogueOperation,
    value: &DialogueValue,
) -> Result<(), DialogueRegistryError> {
    let target = field
        .try_downcast_mut::<String>()
        .ok_or_else(|| DialogueRegistryError::TypeMismatch("string".to_string()))?;
    match op {
        DialogueOperation::Set => {
            let DialogueValue::String(v) = value else {
                return Err(DialogueRegistryError::TypeMismatch("string".to_string()));
            };
            *target = v.clone();
        }
        _ => {
            return Err(DialogueRegistryError::InvalidOperation(
                "string".to_string(),
            ));
        }
    }
    Ok(())
}

fn apply_enum(
    field: &mut dyn bevy::reflect::PartialReflect,
    variants: &[String],
    op: &DialogueOperation,
    value: &DialogueValue,
) -> Result<(), DialogueRegistryError> {
    if !matches!(op, DialogueOperation::Set) {
        return Err(DialogueRegistryError::InvalidOperation("enum".to_string()));
    }
    let DialogueValue::Enum(name) = value else {
        return Err(DialogueRegistryError::TypeMismatch("enum".to_string()));
    };
    let index = variants
        .iter()
        .position(|variant| variant == name)
        .ok_or_else(|| DialogueRegistryError::TypeMismatch("enum".to_string()))?;

    let mut dyn_enum = bevy::reflect::DynamicEnum::new_with_index(
        index,
        name.clone(),
        bevy::reflect::DynamicVariant::Unit,
    );
    dyn_enum.set_represented_type(field.get_represented_type_info());
    field.apply(&dyn_enum);
    Ok(())
}

fn apply_list(
    field: &mut dyn bevy::reflect::PartialReflect,
    inner: &DialogueFieldKind,
    op: &DialogueOperation,
    value: &DialogueValue,
) -> Result<(), DialogueRegistryError> {
    let list = field
        .reflect_mut()
        .as_list()
        .ok()
        .ok_or_else(|| DialogueRegistryError::TypeMismatch("list".to_string()))?;
    match op {
        DialogueOperation::Set => {
            let DialogueValue::List(values) = value else {
                return Err(DialogueRegistryError::TypeMismatch("list".to_string()));
            };
            while list.pop().is_some() {}
            for value in values {
                list.push(value_to_reflect(value, inner)?);
            }
        }
        DialogueOperation::Push => {
            let boxed = value_to_reflect(value, inner)?;
            list.push(boxed);
        }
        DialogueOperation::Remove => {
            let target = value;
            let mut remove_index = None;
            for (index, item) in list.iter().enumerate() {
                let Some(candidate) = reflect_to_value(item, inner) else {
                    continue;
                };
                if &candidate == target {
                    remove_index = Some(index);
                    break;
                }
            }
            if let Some(index) = remove_index {
                let _ = list.remove(index);
            }
        }
        DialogueOperation::Clear => while list.pop().is_some() {},
        _ => return Err(DialogueRegistryError::InvalidOperation("list".to_string())),
    }
    Ok(())
}

fn apply_array(
    field: &mut dyn bevy::reflect::PartialReflect,
    inner: &DialogueFieldKind,
    len: usize,
    op: &DialogueOperation,
    value: &DialogueValue,
) -> Result<(), DialogueRegistryError> {
    let DialogueValue::List(values) = value else {
        return Err(DialogueRegistryError::TypeMismatch("array".to_string()));
    };
    if !matches!(op, DialogueOperation::Set) {
        return Err(DialogueRegistryError::InvalidOperation("array".to_string()));
    }
    if values.len() != len {
        return Err(DialogueRegistryError::TypeMismatch("array".to_string()));
    }
    let array = field
        .reflect_mut()
        .as_array()
        .ok()
        .ok_or_else(|| DialogueRegistryError::TypeMismatch("array".to_string()))?;
    for (index, value) in values.iter().enumerate() {
        let Some(slot) = array.get_mut(index) else {
            return Err(DialogueRegistryError::TypeMismatch("array".to_string()));
        };
        let boxed = value_to_reflect(value, inner)?;
        slot.apply(boxed.as_ref());
    }
    Ok(())
}

fn value_to_reflect(
    value: &DialogueValue,
    kind: &DialogueFieldKind,
) -> Result<Box<dyn bevy::reflect::PartialReflect>, DialogueRegistryError> {
    match (kind, value) {
        (DialogueFieldKind::Bool, DialogueValue::Bool(v)) => Ok(Box::new(*v)),
        (DialogueFieldKind::Int, DialogueValue::Int(v)) => Ok(Box::new(*v)),
        (DialogueFieldKind::Float, DialogueValue::Float(v)) => Ok(Box::new(*v)),
        (DialogueFieldKind::String, DialogueValue::String(v)) => Ok(Box::new(v.clone())),
        (DialogueFieldKind::Enum { variants }, DialogueValue::Enum(name)) => {
            let index = variants
                .iter()
                .position(|variant| variant == name)
                .ok_or_else(|| DialogueRegistryError::TypeMismatch("enum".to_string()))?;
            let dyn_enum = bevy::reflect::DynamicEnum::new_with_index(
                index,
                name.clone(),
                bevy::reflect::DynamicVariant::Unit,
            );
            Ok(Box::new(dyn_enum))
        }
        (DialogueFieldKind::List(inner), DialogueValue::List(values)) => {
            let mut list = bevy::reflect::DynamicList::default();
            for value in values {
                list.push_box(value_to_reflect(value, inner)?);
            }
            Ok(Box::new(list))
        }
        (DialogueFieldKind::Array { element, .. }, DialogueValue::List(values)) => {
            let mut boxed: Vec<Box<dyn bevy::reflect::PartialReflect>> = Vec::new();
            for value in values {
                boxed.push(value_to_reflect(value, element)?);
            }
            Ok(Box::new(bevy::reflect::DynamicArray::new(
                boxed.into_boxed_slice(),
            )))
        }
        _ => Err(DialogueRegistryError::TypeMismatch("value".to_string())),
    }
}
