use bevy::prelude::*;
use funkus_dialogue_editor::DialogueEditorPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(DialogueEditorPlugin::default())
        .run();
}
