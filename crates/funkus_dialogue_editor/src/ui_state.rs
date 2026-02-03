use bevy::prelude::Resource;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeftPanelTab {
    Assets,
    Nodes,
}

impl Default for LeftPanelTab {
    fn default() -> Self {
        Self::Assets
    }
}

#[derive(Resource, Debug, Default)]
pub struct EditorUiState {
    pub left_tab: LeftPanelTab,
    pub asset_filter: String,
    pub node_filter: String,
}
