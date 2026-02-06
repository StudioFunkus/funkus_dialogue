use bevy::prelude::*;
use funkus_dialogue_core::{DialoguePlugin, DialogueRegistryAppExt, DialogueResource};
use funkus_dialogue_editor::DialogueEditorPlugin;

#[derive(Debug, Clone, Reflect, PartialEq, Eq)]
enum ExampleItem {
    Map,
    Potion,
    Key,
}

#[derive(Debug, Clone, Reflect, PartialEq, Eq)]
enum ExampleMood {
    Neutral,
    Happy,
    Angry,
}

#[derive(Resource, Reflect, DialogueResource)]
#[dialogue(key = "game")]
struct ExampleState {
    gold: i32,
    reputation: f32,
    met_npc: bool,
    title: String,
    inventory: Vec<ExampleItem>,
    mood: ExampleMood,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(DialoguePlugin)
        .register_dialogue_resource::<ExampleState>()
        .insert_resource(ExampleState {
            gold: 100,
            reputation: 0.25,
            met_npc: false,
            title: "Stranger".to_string(),
            inventory: vec![ExampleItem::Map],
            mood: ExampleMood::Neutral,
        })
        .add_plugins(DialogueEditorPlugin::default())
        .run();
}
