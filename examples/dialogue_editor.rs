use bevy::prelude::*;
use funkus_dialogue_core::DialoguePlugin;
use funkus_dialogue_editor::FunkusDialogueEditorPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Funkus Dialogue Editor".to_string(),
                resolution: (1280.0, 720.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(DialoguePlugin)
        .add_plugins(FunkusDialogueEditorPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    
    commands.spawn((
        Text::new(
            "Press F2 to toggle the Dialogue Editor\n\
             Right-click in the editor to add nodes\n\
             Drag nodes to move them\n\
             Click and drag from output to input pins to connect nodes"
        ),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));
}