use bevy::prelude::*;
use bevy::prelude::{Message, MessageReader};
use funkus_dialogue_core::graph::{DialogueGraph, DialogueNode};
use std::path::{Path, PathBuf};

/// In-memory representation of a dialogue asset that is currently open in the
/// editor.
#[derive(Clone, Debug)]
pub struct OpenDialogue {
    pub display_name: String,
    pub graph: DialogueGraph,
    pub dirty: bool,
    pub source_path: Option<PathBuf>,
}

impl OpenDialogue {
    #[must_use]
    pub fn new(display_name: impl Into<String>, graph: DialogueGraph) -> Self {
        Self {
            display_name: display_name.into(),
            graph,
            dirty: false,
            source_path: None,
        }
    }

    #[must_use]
    pub fn from_path(path: PathBuf) -> Self {
        let display_name = display_name_for_path(&path);
        let mut graph = DialogueGraph::new().with_name(display_name.clone());

        let start = graph.add_node(DialogueNode::text(format!(
            "This is a placeholder dialogue for {}.",
            display_name
        )));
        let _ = graph.set_start_node(start);

        Self {
            display_name,
            graph,
            dirty: false,
            source_path: Some(path),
        }
    }

    pub fn set_source_path(&mut self, path: PathBuf) {
        self.source_path = Some(path.clone());
        self.display_name = display_name_for_path(&path);
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
    pub available_assets: Vec<PathBuf>,
    pub selected_index: Option<usize>,
    pub path_input: String,
}

impl Default for EditorAssetBrowser {
    fn default() -> Self {
        let available_assets = default_stub_assets();
        let selected_index = if available_assets.is_empty() {
            None
        } else {
            Some(0)
        };
        let path_input = selected_index
            .and_then(|index| available_assets.get(index))
            .map(|path| path.display().to_string())
            .unwrap_or_default();

        Self {
            available_assets,
            selected_index,
            path_input,
        }
    }
}

impl EditorAssetBrowser {
    pub fn refresh_stub_assets(&mut self) {
        if self.available_assets.is_empty() {
            self.available_assets = default_stub_assets();
            self.selected_index = if self.available_assets.is_empty() {
                None
            } else {
                Some(0)
            };
            self.path_input = self
                .selected_index
                .and_then(|index| self.available_assets.get(index))
                .map(|path| path.display().to_string())
                .unwrap_or_default();
        }
    }

    #[must_use]
    pub fn selected_path(&self) -> Option<PathBuf> {
        self.selected_index
            .and_then(|index| self.available_assets.get(index))
            .cloned()
    }

    pub fn select_path(&mut self, path: &Path) {
        if let Some(existing) = self
            .available_assets
            .iter()
            .position(|stored| stored == path)
        {
            self.selected_index = Some(existing);
        } else {
            self.available_assets.push(path.to_path_buf());
            self.selected_index = Some(self.available_assets.len() - 1);
        }
        self.path_input = path.display().to_string();
    }
}

fn default_stub_assets() -> Vec<PathBuf> {
    [
        "assets/dialogue/tutorial.dialogue.json",
        "assets/dialogue/intro.dialogue.json",
        "assets/dialogue/example.dialogue.json",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect()
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
