use bevy::{
    prelude::{
        App, Commands, Component, Entity, EventReader, EventWriter, IntoSystemConfig, Plugin,
        Query, Res, With, Without,
    },
    reflect::Reflect,
    time::{Time, Timer, TimerMode},
};
use tracing::info;

use crate::terrain::{TerrainSet, TileDamageEvent, TileDestroyedEvent};

use super::{
    action::{register_action, CompletedAction, StartedAction},
    move_to::{travel_to_entity, travel_to_nearby_tiles, TravelAction},
};

pub struct DigPlugin;

impl Plugin for DigPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Dig>()
            .register_type::<DigTimer>()
            .add_systems((
                travel_to_nearby_tiles::<Dig>,
                start_digging,
                dig_timer.before(TerrainSet),
                finish_digging,
            ));

        register_action::<Dig, Digging>(app);
    }
}

#[derive(Component, Debug, Clone, Reflect)]
pub struct Dig(pub Entity);

#[derive(Component, Debug, Reflect, Default)]
pub struct TravellingToDigSite;

#[derive(Component, Debug, Reflect, Default)]
pub struct AtDigSite;

impl TravelAction for Dig {
    type TravelingToTarget = TravellingToDigSite;
    type AtTarget = AtDigSite;

    fn target_entity(&self) -> Entity {
        self.0
    }
}

#[derive(Component, Debug, Default, Reflect)]
pub struct Digging;

#[derive(Component, Debug, Reflect)]
pub struct DigTimer(pub Timer);

fn start_digging(
    mut commands: Commands,
    dig_action_query: Query<
        (Entity, &Dig),
        (With<AtDigSite>, With<StartedAction>, Without<DigTimer>),
    >,
) {
    for (dig_action_entity, Dig(tile_entity)) in &dig_action_query {
        info!(action = ?dig_action_entity, tile = ?tile_entity, "Starting digging");
        commands
            .entity(dig_action_entity)
            .insert(DigTimer(Timer::from_seconds(1., TimerMode::Repeating)));
    }
}

fn dig_timer(
    time: Res<Time>,
    mut dig_action_query: Query<(&Dig, &mut DigTimer)>,
    mut tile_damage_event_writer: EventWriter<TileDamageEvent>,
) {
    for (Dig(tile_entity), mut dig_timer) in &mut dig_action_query {
        if dig_timer.0.tick(time.delta()).just_finished() {
            info!(tile_entity = ?tile_entity, "Digging tick");
            tile_damage_event_writer.send(TileDamageEvent {
                tile: *tile_entity,
                damage: 20,
            });
        }
    }
}

fn finish_digging(
    mut commands: Commands,
    mut tile_destroyed_event_reader: EventReader<TileDestroyedEvent>,
    digging_action_query: Query<(Entity, &Dig)>,
) {
    for tile_destroyed_event in tile_destroyed_event_reader.iter() {
        if let Some((digging_action_entity, Dig(tile_entity))) = digging_action_query
            .iter()
            .find(|(_, Dig(tile_entity))| *tile_entity == tile_destroyed_event.entity)
        {
            commands
                .entity(digging_action_entity)
                .remove::<StartedAction>()
                .remove::<DigTimer>()
                .insert(CompletedAction);

            info!(action = ?digging_action_entity, tile = ?tile_entity, "Digging complete");
        }
    }
}
