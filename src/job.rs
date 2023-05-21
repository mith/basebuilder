use bevy::{math::Vec4Swizzles, prelude::*};
use bevy_ecs_tilemap::{prelude::TilemapTileSize, tiles::TilePos};
use bevy_rapier2d::prelude::{KinematicCharacterController, KinematicCharacterControllerOutput};

use crate::terrain::Terrain;

pub struct JobPlugin;

impl Plugin for JobPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems((assign_job, stuck, stuck_timer));
    }
}

#[derive(Component)]
pub struct Job;

#[derive(Component)]
pub struct Worker;

#[derive(Component)]
pub struct HasJob(Entity);

#[derive(Component)]
pub struct AssignedTo {
    pub entity: Entity,
}

#[derive(Component)]
struct UnassignedWorkers(Vec<Entity>);

fn assign_job(
    mut commands: Commands,
    unassigned_job_query: Query<
        (Entity, &TilePos, Option<&UnassignedWorkers>),
        (With<Job>, Without<AssignedTo>),
    >,
    worker_query: Query<(Entity, &GlobalTransform), (With<Worker>, Without<HasJob>)>,
    terrain_query: Query<(&GlobalTransform, &TilemapTileSize), With<Terrain>>,
) {
    let available_workers = worker_query.iter().collect::<Vec<_>>();
    // Look for unnassigned jobs and assign them to the closest unnoccupied worker
    for (job_entity, job_tilepos, opt_unassigned_workers) in &unassigned_job_query {
        let (terrain_transform, tile_size) = terrain_query.single();
        let job_translation = (terrain_transform.compute_matrix()
            * Vec4::new(
                job_tilepos.x as f32 * tile_size.x,
                job_tilepos.y as f32 * tile_size.y,
                0.,
                1.,
            ))
        .xyz();
        // find closest worker
        // first calculate distance to job for each worker and sort
        let mut workers_by_distance = available_workers
            .iter()
            .map(|(entity, transform)| {
                let distance = transform.translation().distance(job_translation);
                (entity, distance)
            })
            .collect::<Vec<_>>();
        workers_by_distance.sort_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap());

        for ((available_worker, _worker_transform)) in workers_by_distance.iter() {
            // check if worker has not been unassigned from this job earlier
            if let Some(unassigned_workers) = opt_unassigned_workers {
                if unassigned_workers.0.contains(available_worker) {
                    continue;
                }
            }
            commands
                .entity(**available_worker)
                .insert(HasJob(job_entity));
            commands.entity(job_entity).insert(AssignedTo {
                entity: **available_worker,
            });
            break;
        }
    }
}

#[derive(Component)]
struct StuckTimer(Timer);

fn stuck(
    mut commands: Commands,
    worker_query: Query<
        (
            Entity,
            Option<&StuckTimer>,
            &KinematicCharacterControllerOutput,
        ),
        (With<Worker>, With<HasJob>),
    >,
) {
    for (worker_entity, opt_stuck_timer, controller_output) in &worker_query {
        let standing_still = controller_output.effective_translation.x == 0.
            && controller_output.effective_translation.y == 0.;
        if !standing_still && opt_stuck_timer.is_some() {
            commands.entity(worker_entity).remove::<StuckTimer>();
        }
        if standing_still && opt_stuck_timer.is_none() {
            commands
                .entity(worker_entity)
                .insert(StuckTimer(Timer::from_seconds(1., TimerMode::Once)));
        }
    }
}

fn stuck_timer(
    mut commands: Commands,
    time: Res<Time>,
    mut stuck_timer_query: Query<(Entity, &mut StuckTimer, &HasJob), With<Worker>>,
    mut unassigned_job_query: Query<&mut UnassignedWorkers, With<Job>>,
) {
    for (worker_entity, mut stuck_timer, has_job) in &mut stuck_timer_query {
        if stuck_timer.0.tick(time.delta()).just_finished() {
            commands.entity(worker_entity).remove::<StuckTimer>();
            commands.entity(worker_entity).remove::<HasJob>();

            commands.entity(has_job.0).remove::<AssignedTo>();
            if let Ok(mut unassigned_workers) = unassigned_job_query.get_mut(has_job.0) {
                unassigned_workers.0.push(worker_entity);
            } else {
                commands
                    .entity(has_job.0)
                    .insert(UnassignedWorkers(vec![worker_entity]));
            }
        }
    }
}
