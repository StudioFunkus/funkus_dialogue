use crate::plugin::INLINE_BADGES_PRESENTATION_KEY;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChoicePresentationMode {
    StandardList,
    InlineBadges,
    External,
}

impl ChoicePresentationMode {
    #[must_use]
    pub(crate) fn supports_builtin_input(self) -> bool {
        matches!(
            self,
            ChoicePresentationMode::StandardList | ChoicePresentationMode::InlineBadges
        )
    }
}

#[must_use]
pub(crate) fn resolve_choice_presentation(key: Option<&str>) -> ChoicePresentationMode {
    match key {
        None => ChoicePresentationMode::StandardList,
        Some(INLINE_BADGES_PRESENTATION_KEY) => ChoicePresentationMode::InlineBadges,
        Some(_) => ChoicePresentationMode::External,
    }
}
