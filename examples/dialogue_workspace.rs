//! Comprehensive editor + preview example.

use bevy::prelude::*;
use bevy::window::WindowResolution;
use funkus_dialogue_core::*;
use funkus_dialogue_editor::{
    DialogueEditorPlugin, DialogueEditorWorkspace, EditorCommand, EditorVisibility,
};
use funkus_dialogue_ui::*;
use std::time::Duration;

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
#[dialogue(key = "example_state")]
struct ExampleState {
    gold: i32,
    reputation: f32,
    met_npc: bool,
    title: String,
    inventory: Vec<ExampleItem>,
    mood: ExampleMood,
}

#[derive(Message, Clone, Debug, Reflect, DialogueMessage)]
#[dialogue(key = "example_dialogue_message")]
struct ExampleDialogueMessage {
    gold_delta: i32,
    new_title: String,
    mood: ExampleMood,
}

impl Default for ExampleState {
    fn default() -> Self {
        Self {
            gold: 100,
            reputation: 0.25,
            met_npc: false,
            title: "Stranger".to_string(),
            inventory: vec![ExampleItem::Map],
            mood: ExampleMood::Neutral,
        }
    }
}

#[derive(Resource, Default)]
struct PreviewContext {
    runner: Option<Entity>,
    custom_overlay_root: Option<Entity>,
    handle: Option<Handle<DialogueAsset>>,
}

#[derive(Resource, Default)]
struct PreviewRequest {
    requested: bool,
}

#[derive(Resource)]
struct MessageDebugVisual {
    last_message: Option<String>,
    active_timer: Timer,
}

impl Default for MessageDebugVisual {
    fn default() -> Self {
        let mut timer = Timer::from_seconds(2.5, TimerMode::Once);
        timer.pause();
        Self {
            last_message: None,
            active_timer: timer,
        }
    }
}

impl MessageDebugVisual {
    fn trigger(&mut self, summary: String) {
        self.last_message = Some(summary);
        self.active_timer.set_duration(Duration::from_secs_f32(2.5));
        self.active_timer.reset();
        self.active_timer.unpause();
    }

    fn tick(&mut self, delta: Duration) {
        if self.active_timer.is_paused() {
            return;
        }
        self.active_timer.tick(delta);
        if self.active_timer.is_finished() {
            self.active_timer.pause();
        }
    }

    fn is_active(&self) -> bool {
        !self.active_timer.is_paused() && !self.active_timer.is_finished()
    }
}

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
enum AppState {
    #[default]
    Editor,
    Preview,
}

const ACTION_OVERLAY_PRESENTATION_KEY: &str = "action_overlay";

struct ActionOverlayPresentation;

impl DialogueChoicePresentation for ActionOverlayPresentation {
    fn key() -> &'static str {
        ACTION_OVERLAY_PRESENTATION_KEY
    }

    fn label() -> &'static str {
        "Action Overlay"
    }

    fn description() -> Option<&'static str> {
        Some("Uses a custom in-game overlay and semantic action key input")
    }
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Dialogue Workspace Example".to_string(),
                    resolution: WindowResolution::new(1280, 720),
                    ..default()
                }),
                ..default()
            }),
            DialoguePlugin,
            DialogueUIPlugin,
            DialogueEditorPlugin::default(),
        ))
        .register_choice_presentation::<ActionOverlayPresentation>()
        .insert_resource(ExampleState::default())
        .init_resource::<MessageDebugVisual>()
        .init_resource::<PreviewContext>()
        .init_resource::<PreviewRequest>()
        .init_state::<AppState>()
        .add_systems(
            Startup,
            (
                spawn_workspace_camera,
                spawn_state_debug_ui,
                load_default_workspace_dialogue,
            ),
        )
        .add_systems(Update, apply_example_dialogue_messages)
        .add_systems(Update, update_state_debug_ui)
        .add_systems(Update, update_message_debug_ui)
        .add_systems(Update, editor_controls.run_if(in_state(AppState::Editor)))
        .add_systems(Update, begin_preview.run_if(in_state(AppState::Editor)))
        .add_systems(Update, preview_input.run_if(in_state(AppState::Preview)))
        .add_systems(
            Update,
            action_overlay_input.run_if(in_state(AppState::Preview)),
        )
        .add_systems(
            Update,
            update_action_overlay_ui.run_if(in_state(AppState::Preview)),
        )
        .add_systems(
            Update,
            handle_preview_end.run_if(in_state(AppState::Preview)),
        )
        .add_systems(OnEnter(AppState::Preview), enter_preview)
        .add_systems(OnExit(AppState::Preview), exit_preview)
        .run();
}

fn spawn_workspace_camera(mut commands: Commands) {
    commands.spawn((Camera::default(), Camera2d));
}

fn load_default_workspace_dialogue(mut command_writer: MessageWriter<EditorCommand>) {
    command_writer.write(EditorCommand::LoadDialogueFromPath {
        path: "example.dialogue.json".into(),
    });
}

fn apply_example_dialogue_messages(
    mut messages: MessageReader<ExampleDialogueMessage>,
    mut state: ResMut<ExampleState>,
    mut visual: ResMut<MessageDebugVisual>,
) {
    for message in messages.read() {
        state.gold += message.gold_delta;
        state.title = message.new_title.clone();
        state.mood = message.mood.clone();
        visual.trigger(format!(
            "gold_delta={}, new_title=\"{}\", mood={:?}",
            message.gold_delta, message.new_title, message.mood
        ));
    }
}

fn editor_controls(mut request: ResMut<PreviewRequest>, keyboard: Res<ButtonInput<KeyCode>>) {
    if keyboard.just_pressed(KeyCode::F5) {
        request.requested = true;
    }
}

fn begin_preview(
    mut commands: Commands,
    mut request: ResMut<PreviewRequest>,
    mut preview: ResMut<PreviewContext>,
    workspace: Res<DialogueEditorWorkspace>,
    mut assets: ResMut<Assets<DialogueAsset>>,
    mut start_events: MessageWriter<StartDialogue>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if !request.requested {
        return;
    }
    request.requested = false;

    let Some(active) = workspace.active_dialogue() else {
        return;
    };

    let handle = assets.add(DialogueAsset::new(active.graph.clone()));
    let entity = commands
        .spawn((Name::new("Preview Dialogue"), DialogueRunner::default()))
        .id();

    start_events.write(StartDialogue {
        entity,
        dialogue_handle: handle.clone(),
    });

    preview.runner = Some(entity);
    preview.handle = Some(handle);
    next_state.set(AppState::Preview);
}

fn enter_preview(
    mut commands: Commands,
    mut visibility: ResMut<EditorVisibility>,
    mut preview: ResMut<PreviewContext>,
) {
    visibility.enabled = false;
    preview.custom_overlay_root = Some(spawn_custom_overlay_ui(&mut commands));
}

fn exit_preview(
    mut commands: Commands,
    mut visibility: ResMut<EditorVisibility>,
    mut preview: ResMut<PreviewContext>,
) {
    visibility.enabled = true;
    if let Some(entity) = preview.custom_overlay_root.take() {
        commands
            .entity(entity)
            .despawn_related::<Children>()
            .despawn();
    }
    if let Some(entity) = preview.runner.take() {
        commands
            .entity(entity)
            .despawn_related::<Children>()
            .despawn();
    }
    preview.handle = None;
}

fn spawn_custom_overlay_ui(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            Text::new(""),
            TextFont {
                font_size: 18.0,
                ..default()
            },
            TextColor(Color::srgb(0.95, 0.85, 0.6)),
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(280.0),
                left: Val::Px(120.0),
                right: Val::Px(120.0),
                display: Display::None,
                ..default()
            },
            ActionOverlayText,
        ))
        .id()
}

fn preview_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    preview: Res<PreviewContext>,
    mut select_events: MessageWriter<SelectDialogueChoice>,
    mut stop_events: MessageWriter<StopDialogue>,
    dialogue_assets: Res<Assets<DialogueAsset>>,
    runners: Query<&DialogueRunner>,
) {
    let Some(entity) = preview.runner else {
        return;
    };
    let Ok(runner) = runners.get(entity) else {
        return;
    };

    if keyboard_input.just_pressed(KeyCode::Escape) {
        stop_events.write(StopDialogue { entity });
    }

    let Some(dialogue) = dialogue_assets.get(&runner.dialogue_handle) else {
        return;
    };

    let is_custom_priority_mode = runner
        .current_choice_presentation_key(dialogue)
        .is_some_and(|key| key == ACTION_OVERLAY_PRESENTATION_KEY);

    if runner.state == DialogueState::WaitingForChoice
        || matches!(runner.state, DialogueState::ChoiceSelected(_))
    {
        if is_custom_priority_mode {
            return;
        }

        let choice_count = runner
            .current_choices(dialogue)
            .map(|choices| choices.len())
            .unwrap_or(0);

        for i in 0..choice_count.min(9) {
            let key = match i {
                0 => KeyCode::Digit1,
                1 => KeyCode::Digit2,
                2 => KeyCode::Digit3,
                3 => KeyCode::Digit4,
                4 => KeyCode::Digit5,
                5 => KeyCode::Digit6,
                6 => KeyCode::Digit7,
                7 => KeyCode::Digit8,
                8 => KeyCode::Digit9,
                _ => continue,
            };
            if keyboard_input.just_pressed(key) {
                select_events.write(SelectDialogueChoice {
                    entity,
                    choice_index: i,
                });
            }
        }
    }
}

fn action_overlay_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    preview: Res<PreviewContext>,
    dialogue_assets: Res<Assets<DialogueAsset>>,
    runners: Query<&DialogueRunner>,
    mut select_events: MessageWriter<SelectDialogueChoice>,
    mut advance_events: MessageWriter<AdvanceDialogue>,
) {
    let Some(entity) = preview.runner else {
        return;
    };
    let Ok(runner) = runners.get(entity) else {
        return;
    };
    if !(runner.state == DialogueState::WaitingForChoice
        || matches!(runner.state, DialogueState::ChoiceSelected(_)))
    {
        return;
    }

    let Some(dialogue) = dialogue_assets.get(&runner.dialogue_handle) else {
        return;
    };
    if runner.current_choice_presentation_key(dialogue) != Some(ACTION_OVERLAY_PRESENTATION_KEY) {
        return;
    }

    let Ok(choices) = runner.current_choices(dialogue) else {
        return;
    };
    if choices.is_empty() {
        return;
    }

    let mut picked_index = None;
    if keyboard_input.just_pressed(KeyCode::Digit1) {
        picked_index = choices
            .iter()
            .find(|choice| choice.choice_key.as_deref() == Some("remove_map"))
            .map(|choice| choice.index);
    } else if keyboard_input.just_pressed(KeyCode::Digit2) {
        picked_index = choices
            .iter()
            .find(|choice| choice.choice_key.as_deref() == Some("clear_inventory"))
            .map(|choice| choice.index);
    } else if keyboard_input.just_pressed(KeyCode::Digit3) {
        picked_index = choices
            .iter()
            .find(|choice| choice.choice_key.as_deref() == Some("set_mood_happy"))
            .map(|choice| choice.index);
    }

    if let Some(choice_index) = picked_index {
        select_events.write(SelectDialogueChoice {
            entity,
            choice_index,
        });
        return;
    }

    let current_index = match runner.state {
        DialogueState::ChoiceSelected(index) => index.min(choices.len().saturating_sub(1)),
        _ => 0,
    };

    if keyboard_input.just_pressed(KeyCode::ArrowUp) {
        let next = if current_index == 0 {
            choices.len() - 1
        } else {
            current_index - 1
        };
        select_events.write(SelectDialogueChoice {
            entity,
            choice_index: next,
        });
    }

    if keyboard_input.just_pressed(KeyCode::ArrowDown) {
        let next = (current_index + 1) % choices.len();
        select_events.write(SelectDialogueChoice {
            entity,
            choice_index: next,
        });
    }

    let confirm_pressed = keyboard_input.just_pressed(KeyCode::Enter)
        || keyboard_input.just_pressed(KeyCode::NumpadEnter)
        || keyboard_input.just_pressed(KeyCode::Space);
    if !confirm_pressed {
        return;
    }

    if matches!(runner.state, DialogueState::ChoiceSelected(_)) {
        advance_events.write(AdvanceDialogue { entity });
    } else {
        select_events.write(SelectDialogueChoice {
            entity,
            choice_index: current_index,
        });
    }
}

fn update_action_overlay_ui(
    preview: Res<PreviewContext>,
    dialogue_assets: Res<Assets<DialogueAsset>>,
    runners: Query<&DialogueRunner>,
    mut overlay_query: Query<(&mut Text, &mut Node), With<ActionOverlayText>>,
) {
    let Ok((mut overlay_text, mut overlay_node)) = overlay_query.single_mut() else {
        return;
    };

    let Some(entity) = preview.runner else {
        overlay_node.display = Display::None;
        *overlay_text = Text::new("");
        return;
    };
    let Ok(runner) = runners.get(entity) else {
        overlay_node.display = Display::None;
        *overlay_text = Text::new("");
        return;
    };
    let Some(dialogue) = dialogue_assets.get(&runner.dialogue_handle) else {
        overlay_node.display = Display::None;
        *overlay_text = Text::new("");
        return;
    };
    if runner.current_choice_presentation_key(dialogue) != Some(ACTION_OVERLAY_PRESENTATION_KEY) {
        overlay_node.display = Display::None;
        *overlay_text = Text::new("");
        return;
    }

    let prompt = match runner.current_node(dialogue) {
        Some(DialogueNode::Choice { prompt, .. }) => prompt
            .clone()
            .unwrap_or_else(|| "Choose a test action".to_string()),
        _ => "Choose a test action".to_string(),
    };

    let Ok(choices) = runner.current_choices(dialogue) else {
        overlay_node.display = Display::None;
        *overlay_text = Text::new("");
        return;
    };
    let selected = match runner.state {
        DialogueState::ChoiceSelected(index) => Some(index),
        _ => None,
    };

    let mut lines = vec![
        "Action Overlay (custom presentation)".to_string(),
        prompt,
        "Pick with 1/2/3, or Arrow Up/Down + Enter/Space".to_string(),
    ];
    for choice in choices {
        let label = choice
            .label
            .clone()
            .unwrap_or_else(|| format!("Choice {}", choice.index + 1));
        let key_hint = match choice.choice_key.as_deref() {
            Some("remove_map") => "1",
            Some("clear_inventory") => "2",
            Some("set_mood_happy") => "3",
            _ => "-",
        };
        let marker = if selected == Some(choice.index) {
            ">"
        } else {
            " "
        };
        lines.push(format!("{marker} [{key_hint}] {label}"));
    }

    overlay_node.display = Display::Flex;
    *overlay_text = Text::new(lines.join("\n"));
}

fn handle_preview_end(
    mut ended: MessageReader<DialogueEnded>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for _ in ended.read() {
        next_state.set(AppState::Editor);
    }
}

#[derive(Component)]
struct StateDebugText;

#[derive(Component)]
struct MessageDebugText;

#[derive(Component)]
struct ActionOverlayText;

fn format_state_debug_text(state: &ExampleState, active_presentation: &str) -> String {
    let inventory = if state.inventory.is_empty() {
        "(empty)".to_string()
    } else {
        state
            .inventory
            .iter()
            .map(|item| format!("{item:?}"))
            .collect::<Vec<_>>()
            .join(", ")
    };

    format!(
        "State\n- gold: {}\n- reputation: {:.2}\n- met_npc: {}\n- title: {}\n- inventory: {}\n- mood: {:?}\n- active_presentation: {}",
        state.gold,
        state.reputation,
        state.met_npc,
        state.title,
        inventory,
        state.mood,
        active_presentation
    )
}

fn spawn_state_debug_ui(mut commands: Commands, state: Res<ExampleState>) {
    commands.spawn((
        Text::new(format_state_debug_text(&state, "none")),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            right: Val::Px(10.0),
            ..default()
        },
        StateDebugText,
    ));
}

fn update_state_debug_ui(
    state: Res<ExampleState>,
    preview: Res<PreviewContext>,
    dialogue_assets: Res<Assets<DialogueAsset>>,
    runners: Query<&DialogueRunner>,
    mut query: Query<&mut Text, With<StateDebugText>>,
) {
    let active_presentation = active_presentation_mode(&preview, &dialogue_assets, &runners);
    let formatted = format_state_debug_text(&state, &active_presentation);
    for mut text in &mut query {
        *text = Text::new(formatted.clone());
    }
}

fn active_presentation_mode(
    preview: &PreviewContext,
    dialogue_assets: &Assets<DialogueAsset>,
    runners: &Query<&DialogueRunner>,
) -> String {
    let Some(entity) = preview.runner else {
        return "none".to_string();
    };
    let Ok(runner) = runners.get(entity) else {
        return "none".to_string();
    };
    let Some(dialogue) = dialogue_assets.get(&runner.dialogue_handle) else {
        return "none".to_string();
    };
    let Some(current_node) = runner.current_node(dialogue) else {
        return "none".to_string();
    };

    if matches!(current_node, DialogueNode::Choice { .. }) {
        runner
            .current_choice_presentation_key(dialogue)
            .unwrap_or("default")
            .to_string()
    } else {
        "none".to_string()
    }
}

fn update_message_debug_ui(
    mut commands: Commands,
    time: Res<Time>,
    mut visual: ResMut<MessageDebugVisual>,
    mut query: Query<(&mut Text, &mut TextColor), With<MessageDebugText>>,
) {
    if query.is_empty() {
        commands.spawn((
            Text::new("Dialogue Message\nNo message triggered yet."),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(Color::srgb(0.6, 0.6, 0.6)),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(200.0),
                right: Val::Px(10.0),
                ..default()
            },
            MessageDebugText,
        ));
    }

    visual.tick(time.delta());

    let (label, color) = if let Some(last) = visual.last_message.as_ref() {
        if visual.is_active() {
            (
                format!("Dialogue Message Triggered\n{last}"),
                Color::srgb(0.4, 1.0, 0.6),
            )
        } else {
            (
                format!("Last Dialogue Message\n{last}"),
                Color::srgb(0.7, 0.7, 0.7),
            )
        }
    } else {
        (
            "Dialogue Message\nNo message triggered yet.".to_string(),
            Color::srgb(0.6, 0.6, 0.6),
        )
    };

    for (mut text, mut text_color) in &mut query {
        *text = Text::new(label.clone());
        *text_color = TextColor(color);
    }
}
