//! Choice presentation metadata registry.
//!
//! Dialogue assets keep presentation selectors as string keys so authoring stays data-driven.
//! The runtime/editor can resolve those keys against this registry to present typed, discoverable
//! options and provide validation/fallback behavior.

use std::collections::HashMap;

use bevy::prelude::*;
use tracing::warn;

/// Metadata describing one registered choice presentation mode.
#[derive(Debug, Clone, Reflect)]
pub struct DialogueChoicePresentationDefinition {
    /// Stable key persisted in dialogue assets.
    pub key: String,
    /// Human-readable label for tooling.
    pub label: String,
    /// Optional longer description for tooling/help panels.
    pub description: Option<String>,
}

impl DialogueChoicePresentationDefinition {
    /// Constructs a new choice presentation definition.
    #[must_use]
    pub fn new(key: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            description: None,
        }
    }

    /// Adds an optional description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Trait for strongly typed registration of choice presentation modes.
pub trait DialogueChoicePresentation: Send + Sync + 'static {
    /// Stable key persisted in dialogue assets.
    fn key() -> &'static str;

    /// Human-readable label for tooling.
    fn label() -> &'static str {
        Self::key()
    }

    /// Optional longer description for tooling/help panels.
    fn description() -> Option<&'static str> {
        None
    }
}

/// Registry of known choice presentation modes.
#[derive(Resource, Default)]
pub struct DialogueChoicePresentationRegistry {
    definitions: HashMap<String, DialogueChoicePresentationDefinition>,
}

impl DialogueChoicePresentationRegistry {
    /// Registers a definition by key.
    pub fn register_definition(&mut self, definition: DialogueChoicePresentationDefinition) {
        let key = definition.key.trim().to_string();
        if key.is_empty() {
            warn!("Ignoring empty dialogue choice presentation key");
            return;
        }

        let mut normalized = definition;
        normalized.key = key.clone();
        if normalized.label.trim().is_empty() {
            normalized.label = normalized.key.clone();
        }

        let replaced = self
            .definitions
            .insert(normalized.key.clone(), normalized)
            .is_some();
        if replaced {
            warn!(
                "Overwriting existing dialogue choice presentation registration for key {}",
                key
            );
        }
    }

    /// Registers a typed presentation mode.
    pub fn register<T: DialogueChoicePresentation>(&mut self) {
        let mut definition = DialogueChoicePresentationDefinition::new(T::key(), T::label());
        if let Some(description) = T::description() {
            definition = definition.with_description(description);
        }
        self.register_definition(definition);
    }

    /// Returns all registered presentations.
    pub fn presentations(&self) -> impl Iterator<Item = &DialogueChoicePresentationDefinition> {
        self.definitions.values()
    }

    /// Looks up a presentation by key.
    #[must_use]
    pub fn presentation(&self, key: &str) -> Option<&DialogueChoicePresentationDefinition> {
        self.definitions.get(key)
    }

    /// Returns `true` if the registry contains the supplied key.
    #[must_use]
    pub fn contains(&self, key: &str) -> bool {
        self.definitions.contains_key(key)
    }
}

/// App extension helpers for registering choice presentation modes.
pub trait DialogueChoicePresentationAppExt {
    /// Registers a typed presentation mode.
    fn register_choice_presentation<T: DialogueChoicePresentation>(&mut self) -> &mut Self;

    /// Registers a definition directly.
    fn register_choice_presentation_definition(
        &mut self,
        definition: DialogueChoicePresentationDefinition,
    ) -> &mut Self;
}

impl DialogueChoicePresentationAppExt for App {
    fn register_choice_presentation<T: DialogueChoicePresentation>(&mut self) -> &mut Self {
        self.world_mut()
            .init_resource::<DialogueChoicePresentationRegistry>();
        self.world_mut()
            .resource_mut::<DialogueChoicePresentationRegistry>()
            .register::<T>();
        self
    }

    fn register_choice_presentation_definition(
        &mut self,
        definition: DialogueChoicePresentationDefinition,
    ) -> &mut Self {
        self.world_mut()
            .init_resource::<DialogueChoicePresentationRegistry>();
        self.world_mut()
            .resource_mut::<DialogueChoicePresentationRegistry>()
            .register_definition(definition);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestPresentation;

    impl DialogueChoicePresentation for TestPresentation {
        fn key() -> &'static str {
            "test_mode"
        }

        fn label() -> &'static str {
            "Test Mode"
        }
    }

    #[test]
    fn registry_registers_typed_presentations() {
        let mut registry = DialogueChoicePresentationRegistry::default();
        registry.register::<TestPresentation>();

        let entry = registry
            .presentation("test_mode")
            .expect("entry should exist");
        assert_eq!(entry.label, "Test Mode");
    }

    #[test]
    fn app_extension_initializes_resource() {
        let mut app = App::new();
        app.register_choice_presentation::<TestPresentation>();

        let registry = app.world().resource::<DialogueChoicePresentationRegistry>();
        assert!(registry.contains("test_mode"));
    }
}
