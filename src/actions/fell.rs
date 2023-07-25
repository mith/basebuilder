use bevy::{
    prelude::{
        App, Commands, Component, Entity, EventReader, EventWriter, IntoSystemConfigs, Plugin,
        Query, Res, Update, With, Without,
    },
    reflect::Reflect,
    time::{Time, Timer, TimerMode},
};
use tracing::info;

use crate::{
    actions::action::{CompletedAction, StartedAction},
    health::HealthDamageEvent,
    tree::TreeDestroyedEvent,
};

use super::{
    action::{register_action, ActionSet},
    move_to::{travel_to_entity, TravelAction},
};

pub struct FellPlugin;

impl Plugin for FellPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Fell>()
            .register_type::<Felling>()
            .add_systems(
                Update,
                (
                    travel_to_entity::<Fell>,
                    start_felling,
                    felling_timer,
                    finish_felling,
                )
                    .before(ActionSet),
            );

        register_action::<Fell, Felling>(app);
    }
}

#[derive(Component, Debug, Reflect, Clone)]
pub struct Fell(pub Entity);

impl TravelAction for Fell {
    type TravelingToTarget = TravelingToTree;
    type AtTarget = AtTree;

    fn target_entity(&self) -> Entity {
        self.0
    }
}

#[derive(Component, Debug, Reflect, Default)]
pub struct Felling;

#[derive(Component, Debug, Default, Reflect)]
pub struct TravelingToTree;

#[derive(Component, Debug, Default, Reflect)]
pub struct AtTree;

#[derive(Component, Debug)]
pub struct FellingTimer(pub Timer);

fn start_felling(
    mut commands: Commands,
    felling_action_query: Query<
        (Entity, &Fell),
        (With<AtTree>, With<StartedAction>, Without<FellingTimer>),
    >,
) {
    for (action_entity, Fell(tree_entity)) in &felling_action_query {
        info!(action=?action_entity, tree=?tree_entity, "Starting felling");

        commands
            .entity(action_entity)
            .insert(FellingTimer(Timer::from_seconds(1.0, TimerMode::Repeating)));
    }
}

fn felling_timer(
    time: Res<Time>,
    mut felling_action_query: Query<(Entity, &Fell, &mut FellingTimer)>,
    mut tree_damage_event_writer: EventWriter<HealthDamageEvent>,
) {
    for (action_entity, Fell(tree_entity), mut felling_timer) in &mut felling_action_query {
        if felling_timer.0.tick(time.delta()).just_finished() {
            info!(action=?action_entity, tree=?tree_entity, "Felling tick");
            tree_damage_event_writer.send(HealthDamageEvent {
                entity: *tree_entity,
                damage: 20,
            });
        }
    }
}

fn finish_felling(
    mut commands: Commands,
    mut tree_destroyed_event_reader: EventReader<TreeDestroyedEvent>,
    felling_action_query: Query<(Entity, &Fell)>,
) {
    for tree_destroyed_event in tree_destroyed_event_reader.iter() {
        if let Some((felling_action_entity, Fell(tree_entity))) = felling_action_query
            .iter()
            .find(|(_, Fell(tree_entity))| tree_destroyed_event.tree == *tree_entity)
        {
            // Mark the action as complete
            commands
                .entity(felling_action_entity)
                .remove::<FellingTimer>()
                .remove::<StartedAction>()
                .insert(CompletedAction);

            info!(felling_action=?felling_action_entity, tree = ?tree_entity, "Felling complete");
        }
    }
}
