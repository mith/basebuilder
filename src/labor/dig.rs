use bevy::prelude::*;
use bevy_ecs_tilemap::tiles::TilePos;

use crate::{
    designation_layer::Designated,
    hovered_tile::HoveredTile,
    labor::job::{
        all_workers_eligible, job_assigned, AssignedJob, AtJobSite, Job, JobSite, Worker,
    },
    terrain::{TerrainParams, TerrainSet, TileDamageEvent, TileDestroyedEvent},
};

use super::job::Complete;

pub struct DigPlugin;

impl Plugin for DigPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<DiggingCompleteEvent>()
            .register_type::<Digging>()
            .register_type::<DiggingTimer>()
            .register_type::<DigToolState>()
            .add_state::<DigToolState>()
            .add_systems((
                designate_dig.run_if(state_exists_and_equals(DigToolState::Designating)),
                job_assigned::<DigJob, Digger>,
                all_workers_eligible::<DigJob>,
                start_digging,
                dig_timer.before(TerrainSet),
                finish_digging,
            ));
    }
}

#[derive(Component)]
pub struct DigJob(Entity);

#[derive(States, Default, Reflect, Clone, Eq, PartialEq, Hash, Debug)]
pub enum DigToolState {
    #[default]
    Inactive,
    Designating,
}

fn designate_dig(
    mut commands: Commands,
    mouse_button_input: Res<Input<MouseButton>>,
    tile_query: Query<(Entity, &TilePos), With<HoveredTile>>,
    terrain: TerrainParams,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        for (tile_entity, tile_pos) in &tile_query {
            let tile_translation = terrain.tile_to_global_pos(*tile_pos);
            let x = tile_translation.x;
            let y = tile_translation.y;
            commands.entity(tile_entity).insert(Designated);
            commands.spawn((
                Job,
                DigJob(tile_entity),
                JobSite(vec![
                    // West
                    Vec2::new(x - 16., y),
                    // East
                    Vec2::new(x + 16., y),
                    // South
                    Vec2::new(x, y - 16.),
                    // Northwest
                    Vec2::new(x - 16., y + 16.),
                    // Southwest
                    Vec2::new(x - 16., y - 16.),
                    // Southeast
                    Vec2::new(x + 16., y - 16.),
                    // Northeast
                    Vec2::new(x + 16., y + 16.),
                ]),
            ));
        }
    }
}

#[derive(Component, Default)]
struct Digger;

#[derive(Component, Reflect)]
pub struct Digging(Entity);

#[derive(Component, Reflect)]
pub struct DiggingTimer(pub Timer);

fn start_digging(
    mut commands: Commands,
    worker_query: Query<(Entity, &AssignedJob), (With<Digger>, Without<Digging>, With<AtJobSite>)>,
    dig_job_query: Query<&DigJob>,
) {
    for (worker_entity, assigned_job) in &worker_query {
        let dig_job = dig_job_query.get(assigned_job.0).unwrap();
        commands.entity(worker_entity).insert((
            Digging(dig_job.0),
            DiggingTimer(Timer::from_seconds(1., TimerMode::Repeating)),
        ));
    }
}

fn dig_timer(
    time: Res<Time>,
    mut digging_worker_query: Query<(&Digging, &mut DiggingTimer), With<Worker>>,
    mut tile_damage_event_writer: EventWriter<TileDamageEvent>,
) {
    for (digging, mut dig_timer) in &mut digging_worker_query {
        if dig_timer.0.tick(time.delta()).just_finished() {
            tile_damage_event_writer.send(TileDamageEvent {
                tile: digging.0,
                damage: 20,
            });
        }
    }
}

pub struct DiggingCompleteEvent {
    pub job: Entity,
    pub parent_job: Option<Entity>,
    pub worker: Entity,
    pub tile: Entity,
}

fn finish_digging(
    mut commands: Commands,
    mut tile_destroyed_event_reader: EventReader<TileDestroyedEvent>,
    digging_worker_query: Query<(&Digging, Entity, &AssignedJob), With<Worker>>,
    parent_job_query: Query<&Parent, With<Job>>,
    mut digging_complete_event_writer: EventWriter<DiggingCompleteEvent>,
) {
    for tile_destroyed_event in tile_destroyed_event_reader.iter() {
        for (digging, worker_entity, assigned_job) in &digging_worker_query {
            if digging.0 == tile_destroyed_event.entity {
                let digging_job = assigned_job.0;
                // Mark the job as complete
                commands.entity(digging_job).insert(Complete);
                // Remove the feller and digging from worker
                commands
                    .entity(worker_entity)
                    .remove::<Digger>()
                    .remove::<Digging>()
                    .remove::<DiggingTimer>();

                // Retrieve the parent if it is a job
                let parent_job = parent_job_query.get(digging_job).ok().map(|p| p.get());

                digging_complete_event_writer.send(DiggingCompleteEvent {
                    job: digging_job,
                    parent_job,
                    worker: worker_entity,
                    tile: tile_destroyed_event.entity,
                });
            }
        }
    }
}
