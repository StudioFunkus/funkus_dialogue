#![deny(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

mod node_editor;
mod state;
mod ui_state;
mod widgets;

use bevy::prelude::*;
use bevy::prelude::{MessageReader, MessageWriter};
use bevy_egui::{
    EguiContext, EguiContextSettings, EguiPlugin, EguiPreUpdateSet, EguiPrimaryContextPass,
    EguiUserTextures, PrimaryEguiContext, egui,
};
use funkus_dialogue_core::graph::DialogueGraph;
use funkus_dialogue_core::registry::{DialogueMessageRegistry, DialogueRegistry};
use funkus_dialogue_core::{DialogueAsset, DialogueEditorMetadata};
use ron::ser::PrettyConfig;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub use state::{
    DialogueEditorWorkspace, EditorAssetBrowser, EditorCommand, EditorPortraitBrowser,
    EditorStatusMessages, EditorVisibility, OpenDialogue, StatusLevel, StatusMessage,
    apply_editor_commands,
};
use ui_state::EditorUiState;
use widgets::{InspectorWidget, LeftPanelWidget, NodeCanvasWidget, StatusBarWidget, ToolbarWidget};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DialogueFileFormat {
    Json,
    Ron,
}

impl DialogueFileFormat {
    fn detect(path: &Path) -> Self {
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase());

        match extension.as_deref() {
            Some("ron") => Self::Ron,
            _ => Self::Json,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Json => "JSON",
            Self::Ron => "RON",
        }
    }
}

#[derive(Debug, Error)]
enum DialogueIoError {
    #[error("Failed to read dialogue file {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to parse JSON dialogue {path}: {source}")]
    ParseJson {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("Failed to parse RON dialogue {path}: {source}")]
    ParseRon {
        path: PathBuf,
        #[source]
        source: ron::error::SpannedError,
    },
    #[error("Failed to create parent directory for {path}: {source}")]
    CreateParent {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to write dialogue file {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to serialize dialogue to JSON for {path}: {source}")]
    SerializeJson {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("Failed to serialize dialogue to RON for {path}: {source}")]
    SerializeRon {
        path: PathBuf,
        #[source]
        source: ron::Error,
    },
    #[error("No save destination set for the active dialogue. Use Save As first.")]
    MissingSaveDestination,
}

fn load_dialogue_from_disk(
    path: &Path,
) -> Result<(DialogueGraph, Option<DialogueEditorMetadata>), DialogueIoError> {
    let format = DialogueFileFormat::detect(path);
    let bytes = fs::read(path).map_err(|source| DialogueIoError::Read {
        path: path.to_path_buf(),
        source,
    })?;

    match format {
        DialogueFileFormat::Json => match serde_json::from_slice::<DialogueAsset>(&bytes) {
            Ok(asset) => Ok((asset.graph, asset.editor)),
            Err(asset_err) => serde_json::from_slice::<DialogueGraph>(&bytes)
                .map(|graph| (graph, None))
                .map_err(|_| DialogueIoError::ParseJson {
                    path: path.to_path_buf(),
                    source: asset_err,
                }),
        },
        DialogueFileFormat::Ron => match ron::de::from_bytes::<DialogueAsset>(&bytes) {
            Ok(asset) => Ok((asset.graph, asset.editor)),
            Err(asset_err) => ron::de::from_bytes::<DialogueGraph>(&bytes)
                .map(|graph| (graph, None))
                .map_err(|_| DialogueIoError::ParseRon {
                    path: path.to_path_buf(),
                    source: asset_err,
                }),
        },
    }
}

fn save_dialogue_to_disk(
    graph: &DialogueGraph,
    editor: Option<DialogueEditorMetadata>,
    path: &Path,
) -> Result<(), DialogueIoError> {
    let format = DialogueFileFormat::detect(path);

    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|source| DialogueIoError::CreateParent {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    match format {
        DialogueFileFormat::Json => {
            let mut asset = DialogueAsset::new(graph.clone());
            asset.editor = editor;
            let data = serde_json::to_vec_pretty(&asset).map_err(|source| {
                DialogueIoError::SerializeJson {
                    path: path.to_path_buf(),
                    source,
                }
            })?;
            fs::write(path, data).map_err(|source| DialogueIoError::Write {
                path: path.to_path_buf(),
                source,
            })
        }
        DialogueFileFormat::Ron => {
            let config = PrettyConfig::new();
            let mut asset = DialogueAsset::new(graph.clone());
            asset.editor = editor;
            let data = ron::ser::to_string_pretty(&asset, config).map_err(|source| {
                DialogueIoError::SerializeRon {
                    path: path.to_path_buf(),
                    source,
                }
            })?;
            fs::write(path, data).map_err(|source| DialogueIoError::Write {
                path: path.to_path_buf(),
                source,
            })
        }
    }
}

/// Plugin that wires the dialogue editor into an existing Bevy app.
///
/// The editor assumes the same asset root as your `AssetPlugin::file_path`
/// (default: `assets` relative to the working directory). Use
/// [`DialogueEditorPlugin::with_assets_root`] if your asset root differs.
#[derive(Default, Clone)]
pub struct DialogueEditorPlugin {
    /// Optional override for the assets root directory used by the editor.
    pub assets_root: Option<PathBuf>,
}

impl DialogueEditorPlugin {
    #[must_use]
    pub fn with_assets_root(root: impl Into<PathBuf>) -> Self {
        Self {
            assets_root: Some(root.into()),
        }
    }
}

impl Plugin for DialogueEditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DialogueEditorWorkspace>();
        if let Some(root) = self.assets_root.clone() {
            app.insert_resource(EditorAssetBrowser::with_assets_root(root.clone()));
            app.insert_resource(EditorPortraitBrowser::with_assets_root(root));
        } else {
            app.init_resource::<EditorAssetBrowser>();
            app.init_resource::<EditorPortraitBrowser>();
        }
        app.init_resource::<EditorStatusMessages>();
        app.init_resource::<EditorVisibility>();
        app.init_resource::<EditorUiState>();
        app.add_message::<EditorCommand>();
        app.add_plugins(EguiPlugin::default());
        app.add_systems(Startup, setup_editor_camera);
        app.add_systems(
            PreUpdate,
            snap_egui_scale_factor.before(EguiPreUpdateSet::ProcessInput),
        );
        app.add_systems(
            Update,
            (
                apply_editor_commands,
                handle_editor_io_commands.after(apply_editor_commands),
            ),
        );
        app.add_systems(EguiPrimaryContextPass, draw_editor_ui);
    }
}

fn setup_editor_camera(mut commands: Commands) {
    commands.spawn((Camera::default(), Camera2d));
}

fn snap_egui_scale_factor(mut contexts: Query<(&mut EguiContextSettings, &Camera)>) {
    for (mut settings, camera) in &mut contexts {
        let target = camera.target_scaling_factor().unwrap_or(1.0);
        let snapped = target.round().max(1.0);
        settings.scale_factor = snapped / target;
    }
}

fn draw_editor_ui(
    mut contexts: Query<(&mut EguiContext, Option<&PrimaryEguiContext>)>,
    mut workspace: ResMut<DialogueEditorWorkspace>,
    mut asset_browser: ResMut<EditorAssetBrowser>,
    mut portrait_browser: ResMut<EditorPortraitBrowser>,
    mut status: ResMut<EditorStatusMessages>,
    editor_visibility: Res<EditorVisibility>,
    mut ui_state: ResMut<EditorUiState>,
    mut command_writer: MessageWriter<EditorCommand>,
    asset_server: Res<AssetServer>,
    mut egui_textures: ResMut<EguiUserTextures>,
    images: Res<Assets<Image>>,
    registry: Option<Res<DialogueRegistry>>,
    message_registry: Option<Res<DialogueMessageRegistry>>,
) {
    if !editor_visibility.enabled {
        return;
    }

    if let Some((mut ctx, _)) = contexts.iter_mut().find(|(_, primary)| primary.is_some()) {
        let ctx = ctx.get_mut();
        asset_browser.refresh_if_needed();
        portrait_browser.refresh_if_needed();

        let mut toolbar = ToolbarWidget;
        let mut left_panel = LeftPanelWidget;
        let mut node_canvas = NodeCanvasWidget;
        let mut inspector = InspectorWidget;
        let mut status_bar = StatusBarWidget;

        egui::TopBottomPanel::top("editor_toolbar").show(ctx, |ui| {
            toolbar.show(
                ui,
                &workspace,
                &asset_browser,
                &mut status,
                &mut command_writer,
            );
        });

        egui::TopBottomPanel::bottom("editor_status").show(ctx, |ui| {
            status_bar.show(ui, &mut status);
        });

        egui::SidePanel::left("editor_left_panel")
            .min_width(260.0)
            .show(ctx, |ui| {
                let output = left_panel.show(
                    ui,
                    &workspace,
                    &mut asset_browser,
                    &mut ui_state,
                    &mut command_writer,
                );
                if let Some(node_id) = output.selected_node {
                    if let Some(active) = workspace.active_dialogue_mut() {
                        active.node_editor.request_selection(node_id);
                    }
                }
            });

        egui::SidePanel::right("editor_right_panel")
            .min_width(280.0)
            .show(ctx, |ui| {
                if let Some(active) = workspace.active_dialogue_mut() {
                    let output = inspector.show(
                        ui,
                        &mut active.graph,
                        &mut active.node_editor,
                        &mut status,
                        &mut portrait_browser,
                        &asset_server,
                        &mut egui_textures,
                        &images,
                        registry.as_deref(),
                        message_registry.as_deref(),
                    );
                    if output.dirty {
                        active.dirty = true;
                    }
                } else {
                    ui.label("No dialogue loaded.");
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(active) = workspace.active_dialogue_mut() {
                let output = node_canvas.show(ui, &mut active.graph, &mut active.node_editor);
                if output.dirty {
                    active.dirty = true;
                }
            } else {
                ui.label("No dialogue file selected yet.");
            }
        });
    }
}

fn handle_editor_io_commands(
    mut command_reader: MessageReader<EditorCommand>,
    mut workspace: ResMut<DialogueEditorWorkspace>,
    mut asset_browser: ResMut<EditorAssetBrowser>,
    mut status: ResMut<EditorStatusMessages>,
) {
    for command in command_reader.read().cloned() {
        match command {
            EditorCommand::LoadDialogueFromPath { path } => {
                let mut resolved_path = path.clone();
                if !asset_browser.is_within_dialogue_root(&resolved_path) {
                    match asset_browser.import_into_dialogue_root(&resolved_path) {
                        Ok(imported) => {
                            let imported_relative =
                                asset_browser.relative_path_if_within(&imported);
                            status.info(format!(
                                "Copied {} into assets as {}",
                                resolved_path.display(),
                                imported_relative.display()
                            ));
                            asset_browser.mark_needs_refresh();
                            resolved_path = imported;
                        }
                        Err(error) => {
                            status.error(format!(
                                "Failed to import {} into assets: {error}",
                                resolved_path.display()
                            ));
                            error!(
                                "Failed to import dialogue file {}: {error}",
                                resolved_path.display()
                            );
                            continue;
                        }
                    }
                }

                let format = DialogueFileFormat::detect(&resolved_path);
                let relative_path = asset_browser.relative_path_if_within(&resolved_path);
                if let Some(existing) = workspace.open_dialogue_index(&relative_path) {
                    workspace.set_active(existing);
                    asset_browser.select_path(&relative_path);
                    status.info(format!(
                        "Dialogue already open: {} ({})",
                        relative_path.display(),
                        format.label()
                    ));
                    continue;
                }

                match load_dialogue_from_disk(&resolved_path) {
                    Ok((graph, editor)) => {
                        let dialogue =
                            OpenDialogue::from_loaded_graph(relative_path.clone(), graph, editor);
                        workspace.open_dialogue(dialogue);
                        asset_browser.select_path(&relative_path);
                        asset_browser.mark_needs_refresh();
                        status.success(format!(
                            "Loaded {} ({})",
                            relative_path.display(),
                            format.label()
                        ));
                    }
                    Err(error) => {
                        status.error(error.to_string());
                        error!("{error}");
                    }
                }
            }
            EditorCommand::SaveActiveDialogue { destination } => {
                if let Some(dialogue) = workspace.active_dialogue_mut() {
                    let target_path = destination.or_else(|| dialogue.source_path.clone());

                    match target_path {
                        Some(path) => {
                            let absolute = asset_browser.to_absolute_dialogue_path(&path);
                            let format = DialogueFileFormat::detect(&absolute);
                            let editor = dialogue.node_editor.editor_metadata();
                            let editor = (!editor.nodes.is_empty()).then_some(editor);
                            match save_dialogue_to_disk(&dialogue.graph, editor, &absolute) {
                                Ok(()) => {
                                    let stored = asset_browser.relative_path_if_within(&absolute);
                                    dialogue.set_source_path(stored.clone());
                                    dialogue.dirty = false;
                                    asset_browser.select_path(&stored);
                                    asset_browser.mark_needs_refresh();
                                    status.success(format!(
                                        "Saved {} ({})",
                                        stored.display(),
                                        format.label()
                                    ));
                                }
                                Err(error) => {
                                    status.error(error.to_string());
                                    error!("{error}");
                                }
                            }
                        }
                        None => {
                            let error = DialogueIoError::MissingSaveDestination;
                            status.error(error.to_string());
                            warn!("{error}");
                        }
                    }
                } else {
                    status.error("No active dialogue to save.");
                    warn!("SaveActiveDialogue command issued with no active dialogue");
                }
            }
            _ => {}
        }
    }
}
