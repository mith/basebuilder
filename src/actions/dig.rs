use bevy::{
    prelude::{
        App, Commands, Component, Entity, EventReader, EventWriter, GlobalTransform,
        IntoSystemConfigs, Plugin, PreUpdate, Query, Res, Update, With, Without,
    },
    reflect::Reflect,
    time::{Time, Timer, TimerMode},
};
use big_brain::{
    prelude::{ActionBuilder, ActionState},
    thinker::{ActionSpan, Actor},
    BigBrainSet,
};
use tracing::info;

use crate::{
    terrain::{TerrainSet, TileDamageEvent, TileDestroyedEvent},
    util::get_entity_position,
};

pub struct DigPlugin;

impl Plugin for DigPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Dig>()
            .register_type::<DigTimer>()
            .add_systems(PreUpdate, dig.in_set(BigBrainSet::Actions))
            .add_systems(Update, dig_timer.before(TerrainSet));
    }
}

#[derive(Component, Debug, Clone, Reflect, ActionBuilder)]
pub struct Dig(pub Entity);

#[derive(Component, Debug, Reflect)]
pub struct DigTimer {
    pub tile_entity: Entity,
    pub timer: Timer,
}

fn dig(
    mut commands: Commands,
    mut dig_action_query: Query<(&Actor, &Dig, &mut ActionState, &ActionSpan)>,
    global_transform_query: Query<&GlobalTransform>,
    mut dig_timer_query: Query<&mut DigTimer>,
    mut tile_destroyed_event_reader: EventReader<TileDestroyedEvent>,
) {
    for (actor, dig, mut action_state, span) in &mut dig_action_query {
        let _guard = span.span().enter();

        match *action_state {
            ActionState::Requested => {
                info!("Starting digging");
                *action_state = ActionState::Executing;
            }
            ActionState::Executing => {
                info!("Digging");
                let actor_position = get_entity_position(&global_transform_query, actor.0);
                let tile_position = get_entity_position(&global_transform_query, dig.0);

                if actor_position.distance(tile_position) < 16. {
                    if let Some(dig_timer) = dig_timer_query.get_mut(actor.0).ok() {
                        if let Some(tile_destroyed_event) = tile_destroyed_event_reader
                            .iter()
                            .find(|event| event.entity == dig_timer.tile_entity)
                        {
                            info!("Digging finished");
                            commands.entity(actor.0).remove::<DigTimer>();
                            *action_state = ActionState::Success;
                        }
                    } else {
                        info!("Digging started");
                        commands.entity(actor.0).insert(DigTimer {
                            tile_entity: dig.0,
                            timer: Timer::from_seconds(1., TimerMode::Repeating),
                        });
                    }
                } else {
                    info!("Too far away to dig");
                    commands.entity(actor.0).remove::<DigTimer>();
                    *action_state = ActionState::Failure;
                }
            }
            ActionState::Cancelled => {
                info!("Digging cancelled");
                commands.entity(actor.0).remove::<DigTimer>();
                *action_state = ActionState::Failure;
            }
            _ => {}
        }
    }
}

fn dig_timer(
    time: Res<Time>,
    mut dig_action_query: Query<&mut DigTimer>,
    mut tile_damage_event_writer: EventWriter<TileDamageEvent>,
) {
    for mut dig_timer in &mut dig_action_query {
        if dig_timer.timer.tick(time.delta()).just_finished() {
            info!(tile_entity = ?dig_timer.tile_entity, "Digging tick");
            tile_damage_event_writer.send(TileDamageEvent {
                tile: dig_timer.tile_entity,
                damage: 20,
            });
        }
    }
}
