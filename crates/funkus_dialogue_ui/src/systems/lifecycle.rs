use std::collections::HashSet;

use bevy::prelude::*;
use funkus_dialogue_core::{DialogueEnded, DialogueStarted};

use crate::components::DialogueDisplay;
use crate::layout::spawn_dialogue_ui;

#[derive(Component)]
pub(crate) struct ManagedDialogueUiRoot;

#[derive(Resource, Default)]
pub(crate) struct DialogueUiLifecycleState {
    active_dialogues: HashSet<Entity>,
}

pub fn mount_dialogue_ui_on_start(
    mut commands: Commands,
    mut started: MessageReader<DialogueStarted>,
    mut lifecycle: ResMut<DialogueUiLifecycleState>,
    existing_roots: Query<Entity, With<DialogueDisplay>>,
) {
    let mut observed_message = false;
    for event in started.read() {
        observed_message = true;
        lifecycle.active_dialogues.insert(event.entity);
    }

    if !observed_message || lifecycle.active_dialogues.is_empty() {
        return;
    }

    if existing_roots.is_empty() {
        let root = spawn_dialogue_ui(&mut commands);
        commands.entity(root).insert(ManagedDialogueUiRoot);
    }
}

pub fn unmount_dialogue_ui_on_end(
    mut commands: Commands,
    mut ended: MessageReader<DialogueEnded>,
    mut lifecycle: ResMut<DialogueUiLifecycleState>,
    managed_roots: Query<Entity, With<ManagedDialogueUiRoot>>,
) {
    let mut observed_message = false;
    for event in ended.read() {
        observed_message = true;
        lifecycle.active_dialogues.remove(&event.entity);
    }

    if !observed_message || !lifecycle.active_dialogues.is_empty() {
        return;
    }

    for root in &managed_roots {
        commands
            .entity(root)
            .despawn_related::<Children>()
            .despawn();
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::message::Messages;
    use bevy::ecs::schedule::common_conditions::on_message;

    use super::*;

    #[test]
    fn mounts_managed_root_when_dialogue_starts() {
        let mut app = App::new();
        app.add_message::<DialogueStarted>()
            .init_resource::<DialogueUiLifecycleState>()
            .add_systems(
                Update,
                mount_dialogue_ui_on_start.run_if(on_message::<DialogueStarted>),
            );

        let runner = app.world_mut().spawn_empty().id();
        app.world_mut()
            .resource_mut::<Messages<DialogueStarted>>()
            .write(DialogueStarted {
                entity: runner,
                start_node_id: funkus_dialogue_core::NodeId::from_raw(1),
            });
        app.update();

        let count = {
            let world = app.world_mut();
            let mut query = world.query_filtered::<Entity, With<DialogueDisplay>>();
            query.iter(world).count()
        };
        assert_eq!(count, 1);
    }

    #[test]
    fn keeps_manual_root_when_dialogue_ends() {
        let mut app = App::new();
        app.add_message::<DialogueStarted>()
            .add_message::<DialogueEnded>()
            .init_resource::<DialogueUiLifecycleState>()
            .add_systems(
                Update,
                (
                    unmount_dialogue_ui_on_end.run_if(on_message::<DialogueEnded>),
                    mount_dialogue_ui_on_start.run_if(on_message::<DialogueStarted>),
                )
                    .chain(),
            );

        let manual = app.world_mut().spawn(DialogueDisplay).id();
        let runner = app.world_mut().spawn_empty().id();

        app.world_mut()
            .resource_mut::<Messages<DialogueStarted>>()
            .write(DialogueStarted {
                entity: runner,
                start_node_id: funkus_dialogue_core::NodeId::from_raw(1),
            });
        app.update();

        app.world_mut()
            .resource_mut::<Messages<DialogueEnded>>()
            .write(DialogueEnded {
                entity: runner,
                normal_exit: true,
            });
        app.update();

        assert!(app.world().get_entity(manual).is_ok());
    }
}
