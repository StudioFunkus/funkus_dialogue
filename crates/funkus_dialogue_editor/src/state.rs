use bevy::prelude::*;
use bevy::prelude::{Message, MessageReader};
use funkus_dialogue_core::graph::DialogueGraph;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::node_editor::DialogueNodeEditorState;
const DEFAULT_ASSETS_DIR: &str = "assets";
const DIALOGUE_SUBDIR: &str = "dialogue";
const PORTRAIT_SUBDIR: &str = "dialogue/portraits";
const SUPPORTED_DIALOGUE_EXTENSIONS: &[&str] = &["json", "ron"];
#[cfg(feature = "image_formats_all")]
pub const SUPPORTED_PORTRAIT_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "bmp", "tga", "gif", "hdr", "exr", "ktx2", "dds", "basis", "webp",
    "tiff", "tif", "ico", "pnm", "qoi", "ff",
];

#[cfg(all(not(feature = "image_formats_all"), feature = "image_formats_basic"))]
pub const SUPPORTED_PORTRAIT_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp"];

#[cfg(all(
    not(feature = "image_formats_all"),
    not(feature = "image_formats_basic")
))]
pub const SUPPORTED_PORTRAIT_EXTENSIONS: &[&str] = &[];

/// In-memory representation of a dialogue asset that is currently open in the
/// editor.
#[derive(Clone, Debug)]
pub struct OpenDialogue {
    pub display_name: String,
    pub graph: DialogueGraph,
    pub node_editor: DialogueNodeEditorState,
    pub dirty: bool,
    pub source_path: Option<PathBuf>,
}

impl OpenDialogue {
    #[must_use]
    pub fn new(display_name: impl Into<String>, graph: DialogueGraph) -> Self {
        let node_editor = DialogueNodeEditorState::from_graph(&graph);
        Self {
            display_name: display_name.into(),
            graph,
            node_editor,
            dirty: false,
            source_path: None,
        }
    }

    #[must_use]
    pub fn from_loaded_graph(path: PathBuf, graph: DialogueGraph) -> Self {
        let display_name = graph
            .name
            .clone()
            .unwrap_or_else(|| display_name_for_path(&path));
        let node_editor = DialogueNodeEditorState::from_graph(&graph);

        Self {
            display_name,
            graph,
            node_editor,
            dirty: false,
            source_path: Some(path),
        }
    }

    pub fn set_source_path(&mut self, path: PathBuf) {
        self.source_path = Some(path);
    }
}

fn display_name_for_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(str::to_owned)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| path.display().to_string())
}

#[derive(Resource, Debug)]
pub struct EditorAssetBrowser {
    pub assets_root: PathBuf,
    pub dialogue_root: PathBuf,
    canonical_dialogue_root: PathBuf,
    pub available_assets: Vec<PathBuf>,
    pub selected_index: Option<usize>,
    pub path_input: String,
    needs_refresh: bool,
}

/// File-system backed list of portrait assets under `assets/dialogue/portraits`.
#[derive(Resource, Debug)]
pub struct EditorPortraitBrowser {
    assets_root: PathBuf,
    portrait_root: PathBuf,
    canonical_portrait_root: PathBuf,
    pub available_assets: Vec<PathBuf>,
    loaded_handles: HashMap<String, Handle<Image>>,
    needs_refresh: bool,
}

impl Default for EditorPortraitBrowser {
    fn default() -> Self {
        Self::new(default_assets_root())
    }
}

impl EditorPortraitBrowser {
    #[must_use]
    pub fn with_assets_root(root: impl Into<PathBuf>) -> Self {
        Self::new(root.into())
    }

    fn new(assets_root: PathBuf) -> Self {
        let portrait_root = assets_root.join(PORTRAIT_SUBDIR);
        let canonical_portrait_root =
            fs::canonicalize(&portrait_root).unwrap_or_else(|_| portrait_root.clone());

        let mut browser = Self {
            assets_root,
            portrait_root,
            canonical_portrait_root,
            available_assets: Vec::new(),
            loaded_handles: HashMap::new(),
            needs_refresh: true,
        };

        if let Err(error) = browser.ensure_portrait_directory() {
            warn!(
                "Failed to create portrait assets directory {}: {error}",
                browser.portrait_root.display()
            );
        }

        browser.refresh_assets();
        browser
    }
    pub fn refresh_assets(&mut self) {
        if let Err(error) = self.ensure_portrait_directory() {
            warn!(
                "Failed to ensure portrait directory {} exists: {error}",
                self.portrait_root.display()
            );
        }

        match collect_portrait_assets(&self.portrait_root, &self.assets_root) {
            Ok(mut assets) => {
                assets.sort();
                self.available_assets = assets;
            }
            Err(error) => {
                warn!(
                    "Failed to scan portrait assets under {}: {error}",
                    self.portrait_root.display()
                );
                self.available_assets.clear();
            }
        }

        let keep: HashSet<String> = self
            .available_assets
            .iter()
            .map(|path| portrait_key_for_path(path))
            .collect();
        self.loaded_handles.retain(|key, _| keep.contains(key));

        self.needs_refresh = false;
    }

    pub fn load_handle(&mut self, asset_server: &AssetServer, path: &str) -> Handle<Image> {
        if let Some(handle) = self.loaded_handles.get(path) {
            return handle.clone();
        }
        let handle = asset_server.load::<Image>(path.to_string());
        self.loaded_handles.insert(path.to_string(), handle.clone());
        handle
    }

    pub fn mark_needs_refresh(&mut self) {
        self.needs_refresh = true;
    }

    pub fn refresh_if_needed(&mut self) {
        if self.needs_refresh {
            self.refresh_assets();
        }
    }

    pub fn import_into_portrait_root(&mut self, external_path: &Path) -> io::Result<PathBuf> {
        self.ensure_portrait_directory()?;

        let file_name = external_path.file_name().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Selected path {} does not have a valid file name.",
                    external_path.display()
                ),
            )
        })?;

        let mut destination = self.portrait_root.join(file_name);
        if destination.exists() {
            let source = Path::new(file_name);
            let stem = source
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("imported");
            let extension = source.extension().and_then(|ext| ext.to_str());

            let mut counter = 1;
            loop {
                let candidate = if let Some(ext) = extension {
                    format!("{stem}-{counter}.{ext}")
                } else {
                    format!("{stem}-{counter}")
                };
                destination = self.portrait_root.join(candidate);
                if !destination.exists() {
                    break;
                }
                counter += 1;
            }
        }

        fs::copy(external_path, &destination)?;
        self.mark_needs_refresh();
        Ok(destination)
    }

    #[must_use]
    pub fn relative_path_if_within_assets(&self, path: &Path) -> PathBuf {
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.assets_root.join(path)
        };
        absolute
            .strip_prefix(&self.assets_root)
            .map(PathBuf::from)
            .unwrap_or_else(|_| path.to_path_buf())
    }

    fn ensure_portrait_directory(&mut self) -> io::Result<()> {
        fs::create_dir_all(&self.portrait_root)?;
        if let Ok(canonical) = fs::canonicalize(&self.portrait_root) {
            self.canonical_portrait_root = canonical;
        }
        Ok(())
    }
}

impl Default for EditorAssetBrowser {
    fn default() -> Self {
        Self::new(default_assets_root())
    }
}

impl EditorAssetBrowser {
    #[must_use]
    pub fn with_assets_root(root: impl Into<PathBuf>) -> Self {
        Self::new(root.into())
    }

    fn new(assets_root: PathBuf) -> Self {
        let dialogue_root = assets_root.join(DIALOGUE_SUBDIR);
        let canonical_dialogue_root =
            fs::canonicalize(&dialogue_root).unwrap_or_else(|_| dialogue_root.clone());

        let mut browser = Self {
            assets_root,
            dialogue_root,
            canonical_dialogue_root,
            available_assets: Vec::new(),
            selected_index: None,
            path_input: String::new(),
            needs_refresh: true,
        };

        if let Err(error) = browser.ensure_dialogue_directory() {
            warn!(
                "Failed to create dialogue assets directory {}: {error}",
                browser.dialogue_root.display()
            );
        }

        browser.refresh_assets();
        browser
    }
    pub fn refresh_assets(&mut self) {
        let previous_selection = self.selected_relative_path();

        if let Err(error) = self.ensure_dialogue_directory() {
            warn!(
                "Failed to ensure dialogue directory {} exists: {error}",
                self.dialogue_root.display()
            );
        }

        match collect_dialogue_assets(&self.dialogue_root) {
            Ok(mut assets) => {
                assets.sort();
                self.available_assets = assets;
            }
            Err(error) => {
                warn!(
                    "Failed to scan dialogue assets under {}: {error}",
                    self.dialogue_root.display()
                );
                self.available_assets.clear();
            }
        }

        if let Some(previous) = previous_selection {
            if let Some(index) = self
                .available_assets
                .iter()
                .position(|candidate| candidate == &previous)
            {
                self.selected_index = Some(index);
                self.path_input = self.available_assets[index].display().to_string();
            } else {
                self.selected_index = None;
                self.path_input.clear();
            }
        } else if let Some(first) = self.available_assets.first() {
            self.selected_index = Some(0);
            self.path_input = first.display().to_string();
        } else {
            self.path_input.clear();
        }

        self.needs_refresh = false;
    }

    #[must_use]
    pub fn selected_path(&self) -> Option<PathBuf> {
        self.selected_index
            .and_then(|index| self.available_assets.get(index))
            .map(|relative| self.dialogue_root.join(relative))
    }

    pub fn select_path(&mut self, path: &Path) {
        let absolute = self.to_absolute_dialogue_path(path);
        if let Ok(relative) = absolute.strip_prefix(&self.dialogue_root) {
            let relative = relative.to_path_buf();
            if let Some(existing) = self
                .available_assets
                .iter()
                .position(|stored| stored == &relative)
            {
                self.selected_index = Some(existing);
            } else {
                self.available_assets.push(relative.clone());
                self.available_assets.sort();
                self.selected_index = self
                    .available_assets
                    .iter()
                    .position(|stored| stored == &relative);
            }
            if let Some(index) = self.selected_index {
                self.path_input = self.available_assets[index].display().to_string();
            } else {
                self.path_input = relative.display().to_string();
            }
        } else {
            // Path outside dialogue root; note it but do not track in the list.
            self.selected_index = None;
            self.path_input = absolute.display().to_string();
        }
    }

    #[must_use]
    pub fn selected_relative_path(&self) -> Option<PathBuf> {
        self.selected_index
            .and_then(|index| self.available_assets.get(index))
            .cloned()
    }

    #[must_use]
    pub fn dialogue_root_display(&self) -> String {
        self.dialogue_root.display().to_string()
    }

    #[must_use]
    pub fn path_input(&self) -> &str {
        &self.path_input
    }

    pub fn mark_needs_refresh(&mut self) {
        self.needs_refresh = true;
    }

    pub fn refresh_if_needed(&mut self) {
        if self.needs_refresh {
            self.refresh_assets();
        }
    }

    #[must_use]
    pub fn is_within_dialogue_root(&self, path: &Path) -> bool {
        let absolute = self.to_absolute_dialogue_path(path);
        let candidate = fs::canonicalize(&absolute).unwrap_or(absolute.clone());
        candidate.starts_with(&self.canonical_dialogue_root)
    }

    pub fn import_into_dialogue_root(&mut self, external_path: &Path) -> io::Result<PathBuf> {
        self.ensure_dialogue_directory()?;

        let file_name = external_path.file_name().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Selected path {} does not have a valid file name.",
                    external_path.display()
                ),
            )
        })?;

        let mut destination = self.dialogue_root.join(file_name);
        if destination.exists() {
            let source = Path::new(file_name);
            let stem = source
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("imported");
            let extension = source.extension().and_then(|ext| ext.to_str());

            let mut counter = 1;
            loop {
                let candidate = if let Some(ext) = extension {
                    format!("{stem}-{counter}.{ext}")
                } else {
                    format!("{stem}-{counter}")
                };
                destination = self.dialogue_root.join(candidate);
                if !destination.exists() {
                    break;
                }
                counter += 1;
            }
        }

        fs::copy(external_path, &destination)?;
        self.mark_needs_refresh();
        Ok(destination)
    }

    #[must_use]
    pub fn relative_path_if_within(&self, path: &Path) -> PathBuf {
        let absolute = self.to_absolute_dialogue_path(path);
        absolute
            .strip_prefix(&self.dialogue_root)
            .map(PathBuf::from)
            .unwrap_or_else(|_| path.to_path_buf())
    }

    #[must_use]
    pub fn to_absolute_dialogue_path(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.dialogue_root.join(path)
        }
    }

    fn ensure_dialogue_directory(&mut self) -> io::Result<()> {
        fs::create_dir_all(&self.dialogue_root)?;
        if let Ok(canonical) = fs::canonicalize(&self.dialogue_root) {
            self.canonical_dialogue_root = canonical;
        }
        Ok(())
    }
}

fn collect_dialogue_assets(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                stack.push(path);
            } else if file_type.is_file() && is_dialogue_file(&path) {
                if let Ok(relative) = path.strip_prefix(root) {
                    files.push(relative.to_path_buf());
                } else {
                    files.push(path);
                }
            }
        }
    }

    files.sort();
    Ok(files)
}

fn collect_portrait_assets(root: &Path, assets_root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                stack.push(path);
            } else if file_type.is_file() && is_portrait_file(&path) {
                if let Ok(relative) = path.strip_prefix(assets_root) {
                    files.push(relative.to_path_buf());
                } else {
                    files.push(path);
                }
            }
        }
    }

    files.sort();
    Ok(files)
}

fn is_dialogue_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            let lower = ext.to_ascii_lowercase();
            SUPPORTED_DIALOGUE_EXTENSIONS
                .iter()
                .any(|candidate| candidate == &lower)
        })
        .unwrap_or(false)
}

fn is_portrait_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            let lower = ext.to_ascii_lowercase();
            SUPPORTED_PORTRAIT_EXTENSIONS
                .iter()
                .any(|candidate| candidate == &lower)
        })
        .unwrap_or(false)
}

fn portrait_key_for_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn default_assets_root() -> PathBuf {
    std::env::current_dir()
        .map(|cwd| cwd.join(DEFAULT_ASSETS_DIR))
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_ASSETS_DIR))
}

/// Tracks the set of open dialogues along with the currently active document.
#[derive(Resource, Default)]
pub struct DialogueEditorWorkspace {
    pub open_dialogues: Vec<OpenDialogue>,
    pub active_index: Option<usize>,
    next_untitled_index: u32,
}

impl DialogueEditorWorkspace {
    /// Creates an empty workspace.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` when the workspace has an active dialogue.
    #[must_use]
    pub fn has_active(&self) -> bool {
        self.active_index
            .map(|idx| idx < self.open_dialogues.len())
            .unwrap_or(false)
    }

    /// Returns a mutable reference to the active dialogue, if one is selected.
    pub fn active_dialogue_mut(&mut self) -> Option<&mut OpenDialogue> {
        let idx = self.active_index?;
        self.open_dialogues.get_mut(idx)
    }

    /// Returns a reference to the active dialogue, if one is selected.
    #[must_use]
    pub fn active_dialogue(&self) -> Option<&OpenDialogue> {
        let idx = self.active_index?;
        self.open_dialogues.get(idx)
    }

    /// Iterates over all open dialogues with their index.
    pub fn iter_dialogues(&self) -> impl Iterator<Item = (usize, &OpenDialogue)> {
        self.open_dialogues.iter().enumerate()
    }

    /// Adds a new, empty dialogue to the workspace and selects it.
    pub fn open_new_dialogue(&mut self, preferred_name: Option<String>) {
        let name = preferred_name.unwrap_or_else(|| {
            self.next_untitled_index += 1;
            format!("Untitled {}", self.next_untitled_index)
        });

        let dialogue = OpenDialogue::new(name, DialogueGraph::new());
        self.open_dialogue(dialogue);
    }

    /// Adds an existing dialogue to the workspace and selects it.
    pub fn open_dialogue(&mut self, dialogue: OpenDialogue) {
        if let Some(path) = dialogue.source_path.as_ref() {
            if let Some(existing) = self.open_dialogue_index(path) {
                self.active_index = Some(existing);
                return;
            }
        }
        self.open_dialogues.push(dialogue);
        self.active_index = Some(self.open_dialogues.len() - 1);
    }

    /// Attempts to close the active dialogue, respecting the dirty flag unless
    /// `force` is set.
    pub fn close_active_dialogue(&mut self, force: bool) {
        if let Some(idx) = self.active_index {
            if let Some(dialogue) = self.open_dialogues.get(idx) {
                if dialogue.dirty && !force {
                    return;
                }
            }

            self.open_dialogues.remove(idx);
            if self.open_dialogues.is_empty() {
                self.active_index = None;
            } else {
                self.active_index = Some(self.open_dialogues.len() - 1);
            }
        }
    }

    /// Marks the active dialogue as dirty, if one is selected.
    pub fn mark_active_dirty(&mut self) {
        if let Some(dialogue) = self.active_dialogue_mut() {
            dialogue.dirty = true;
        }
    }

    /// Sets the active dialogue by index.
    pub fn set_active(&mut self, index: usize) {
        if index < self.open_dialogues.len() {
            self.active_index = Some(index);
        }
    }

    #[must_use]
    pub fn open_dialogue_index(&self, path: &Path) -> Option<usize> {
        self.open_dialogues.iter().position(|dialogue| {
            dialogue
                .source_path
                .as_ref()
                .is_some_and(|source| source == path)
        })
    }
}

/// Commands that mutate the workspace, processed as a Bevy event stream.
#[derive(Clone, Debug)]
pub enum EditorCommand {
    NewDialogue {
        preferred_name: Option<String>,
    },
    OpenDialogue(OpenDialogue),
    CloseActive {
        force: bool,
    },
    SetActive {
        index: usize,
    },
    MarkActiveDirty,
    LoadDialogueFromPath {
        path: std::path::PathBuf,
    },
    SaveActiveDialogue {
        destination: Option<std::path::PathBuf>,
    },
}

impl Message for EditorCommand {}

/// Applies editor commands to the workspace.
pub fn apply_editor_commands(
    mut workspace: ResMut<DialogueEditorWorkspace>,
    mut commands: MessageReader<EditorCommand>,
) {
    for command in commands.read().cloned() {
        match command {
            EditorCommand::NewDialogue { preferred_name } => {
                workspace.open_new_dialogue(preferred_name);
            }
            EditorCommand::OpenDialogue(dialogue) => {
                workspace.open_dialogue(dialogue);
            }
            EditorCommand::CloseActive { force } => {
                workspace.close_active_dialogue(force);
            }
            EditorCommand::SetActive { index } => {
                workspace.set_active(index);
            }
            EditorCommand::MarkActiveDirty => {
                workspace.mark_active_dirty();
            }
            EditorCommand::LoadDialogueFromPath { .. } => {
                // IO systems will handle this command.
            }
            EditorCommand::SaveActiveDialogue { .. } => {
                // IO systems will handle this command.
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusLevel {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct StatusMessage {
    pub level: StatusLevel,
    pub text: String,
}

impl StatusMessage {
    #[must_use]
    pub fn new(level: StatusLevel, text: impl Into<String>) -> Self {
        Self {
            level,
            text: text.into(),
        }
    }
}

#[derive(Resource, Debug, Default)]
pub struct EditorStatusMessages {
    pub messages: Vec<StatusMessage>,
}

/// Controls whether the editor UI is rendered.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorVisibility {
    pub enabled: bool,
}

impl Default for EditorVisibility {
    fn default() -> Self {
        Self { enabled: true }
    }
}

impl EditorStatusMessages {
    pub fn push(&mut self, level: StatusLevel, text: impl Into<String>) {
        self.messages.push(StatusMessage::new(level, text));
        const MAX_MESSAGES: usize = 20;
        if self.messages.len() > MAX_MESSAGES {
            let excess = self.messages.len() - MAX_MESSAGES;
            self.messages.drain(0..excess);
        }
    }

    pub fn info(&mut self, text: impl Into<String>) {
        self.push(StatusLevel::Info, text);
    }

    pub fn success(&mut self, text: impl Into<String>) {
        self.push(StatusLevel::Success, text);
    }

    pub fn warning(&mut self, text: impl Into<String>) {
        self.push(StatusLevel::Warning, text);
    }

    pub fn error(&mut self, text: impl Into<String>) {
        self.push(StatusLevel::Error, text);
    }

    pub fn remove(&mut self, index: usize) {
        if index < self.messages.len() {
            self.messages.remove(index);
        }
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }
}
