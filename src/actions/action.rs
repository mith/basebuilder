use std::collections::VecDeque;

use bevy::{
    prelude::{
        apply_system_buffers, Added, App, Children, Commands, Component, DespawnRecursiveExt,
        Entity, EventWriter, IntoSystemConfigs, Parent, Plugin, Query, SystemSet, With, Without,
    },
    reflect::{FromReflect, Reflect},
};
use tracing::info;

pub struct ActionPlugin;

impl Plugin for ActionPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<CompletedAction>()
            .register_type::<Action>()
            .register_type::<ActionQueue>()
            .register_type::<CurrentAction>()
            .add_event::<ActionStartedEvent>()
            .add_event::<ActionSuspensedEvent>()
            .add_systems(
                (apply_system_buffers, start_next_action)
                    .chain()
                    .in_set(ActionSet),
            );
    }
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct ActionSet;

#[derive(Component, Default, Debug, Reflect, FromReflect)]
pub struct Action;

#[derive(Component, Default, Debug, Reflect)]
struct ActionQueue(VecDeque<Entity>);

#[derive(Component, Debug, Clone, Reflect)]
pub struct CurrentAction(pub Entity);

pub fn register_action<ActionType, PerformerType>(app: &mut App)
where
    ActionType: Component + std::clone::Clone,
    PerformerType: Component + std::default::Default,
{
    app.add_event::<ActionCompletedEvent<ActionType>>()
        .add_systems(
            (
                action_started::<ActionType, PerformerType>,
                action_completed::<ActionType, PerformerType>,
                action_suspended::<ActionType>,
            )
                .in_set(ActionSet),
        );
}

#[derive(Component, Debug, Default, Reflect)]
pub struct StartedAction;

pub struct ActionStartedEvent {
    pub action_entity: Entity,
    pub performer_entity: Entity,
}

fn action_started<ActionType, PerformerType>(
    mut commands: Commands,
    started_actions_query: Query<(Entity, &Parent), (With<ActionType>, Added<StartedAction>)>,
    mut action_started_event_writer: EventWriter<ActionStartedEvent>,
) where
    ActionType: Component,
    PerformerType: Component + Default,
{
    for (action_entity, parent) in &started_actions_query {
        let performer_entity = parent.get();
        commands
            .entity(performer_entity)
            .insert(PerformerType::default());

        info!(action = ?action_entity, performer = ?performer_entity, "Action started");
        action_started_event_writer.send(ActionStartedEvent {
            action_entity,
            performer_entity,
        });
    }
}

pub struct ActionCompletedEvent<A: Component> {
    pub action_entity: Entity,
    pub performer_entity: Entity,
    pub action: A,
}

#[derive(Component, Debug, Default, Reflect)]
pub struct CompletedAction;

fn action_completed<ActionType: Component + Clone, PerformerType: Component>(
    mut commands: Commands,
    completed_actions_query: Query<(Entity, &ActionType, &Parent), Added<CompletedAction>>,
    mut action_completed_event_writer: EventWriter<ActionCompletedEvent<ActionType>>,
) {
    for (action_entity, action, parent) in &completed_actions_query {
        let performer_entity = parent.get();
        commands
            .entity(performer_entity)
            .remove::<CurrentAction>()
            .remove::<PerformerType>();
        commands.entity(action_entity).despawn_recursive();
        info!(action = ?action_entity, performer = ?performer_entity, "Action completed");
        action_completed_event_writer.send(ActionCompletedEvent {
            action_entity,
            performer_entity,
            action: action.clone(),
        });
    }
}

#[derive(Component, Debug, Default, Reflect)]
pub struct SuspendedAction;

pub struct ActionSuspensedEvent {
    pub action_entity: Entity,
    pub performer_entity: Entity,
}

fn action_suspended<A: Component>(
    suspended_actions_query: Query<(Entity, &Parent), Added<SuspendedAction>>,
    mut action_suspension_event_writer: EventWriter<ActionSuspensedEvent>,
) {
    for (action_entity, parent) in &suspended_actions_query {
        let performer_entity = parent.get();
        info!(action = ?action_entity, performer = ?performer_entity, "Action suspended");
        action_suspension_event_writer.send(ActionSuspensedEvent {
            action_entity,
            performer_entity,
        });
    }
}

#[derive(Component, Debug, Reflect)]
pub struct BeforeAction(pub Entity);

fn start_next_action(
    mut commands: Commands,
    idle_performer_query: Query<(Entity, &Children), Without<CurrentAction>>,
    action_query: Query<&Action, Without<CompletedAction>>,
) {
    for (performer_entity, children) in &idle_performer_query {
        let mut action_queue = VecDeque::from_iter(
            children
                .iter()
                .filter_map(|entity| action_query.contains(*entity).then(|| *entity)),
        );

        if let Some(action_entity) = action_queue.pop_front() {
            commands
                .entity(performer_entity)
                .insert(CurrentAction(action_entity));
            commands.entity(action_entity).insert(StartedAction);
            info!(action = ?action_entity, performer = ?performer_entity, "Action scheduled to start");
        }
    }
}
