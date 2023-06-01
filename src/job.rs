use bevy::{math::Vec3Swizzles, prelude::*, utils::HashMap};
use bevy_ecs_tilemap::tiles::TilePos;
use bevy_rapier2d::prelude::KinematicCharacterControllerOutput;

use crate::{ai_controller::MoveTo, terrain::TerrainParams};

pub struct JobPlugin;

impl Plugin for JobPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<HasJob>()
            .register_type::<AssignedTo>()
            .register_type::<BlacklistedWorkers>()
            .add_systems((assign_job, blacklist_timer, stuck, stuck_timer));
    }
}

#[derive(Component)]
pub struct Job;

#[derive(Component)]
pub struct Worker;

#[derive(Component, Reflect)]
pub struct HasJob(Entity);

#[derive(Component, Reflect)]
pub struct AssignedTo {
    pub entity: Entity,
}

#[derive(Component, Reflect)]
struct BlacklistedWorkers(HashMap<Entity, Timer>);

#[derive(Component)]
pub struct Accessible;

fn assign_job(
    mut commands: Commands,
    unassigned_job_query: Query<
        (
            Entity,
            Option<&TilePos>,
            Option<&GlobalTransform>,
            Option<&BlacklistedWorkers>,
        ),
        (With<Job>, Without<AssignedTo>, With<Accessible>),
    >,
    worker_query: Query<(Entity, &GlobalTransform), (With<Worker>, Without<HasJob>)>,
    terrain: TerrainParams,
) {
    let mut available_workers = worker_query.iter().collect::<Vec<_>>();
    // Look for unnassigned jobs and assign them to the closest unnoccupied worker
    for (job_entity, opt_job_tilepos, opt_job_transform, opt_blacklisted_workers) in
        &unassigned_job_query
    {
        let Some(job_world_pos) = (
            if let Some(jsob_tile_pos) = opt_job_tilepos {
                Some(terrain.tile_to_global_pos(*jsob_tile_pos))
            } else if let Some(job_transform) = opt_job_transform {
                Some(job_transform.translation().xy())
            } else {
                None
            }
        ) else {
            return;
        };
        // find closest worker
        // first calculate distance to job for each worker and sort
        available_workers.sort_by(|(_, a), (_, b)| {
            a.translation()
                .xy()
                .distance(job_world_pos)
                .partial_cmp(&b.translation().xy().distance(job_world_pos))
                .unwrap()
        });

        let available_worker =
            available_workers
                .iter()
                .enumerate()
                .find(|(_i, (worker_entity, _))| {
                    // check if worker has not been unassigned from this job earlier
                    if let Some(blacklisted_workers) = opt_blacklisted_workers {
                        if blacklisted_workers.0.contains_key(worker_entity) {
                            return false;
                        }
                    }
                    true
                });

        if let Some((i, (available_worker, _worker_transform))) = available_worker {
            commands
                .entity(*available_worker)
                .insert(HasJob(job_entity));
            commands.entity(job_entity).insert(AssignedTo {
                entity: *available_worker,
            });
            available_workers.remove(i);
        }
    }
}

fn blacklist_timer(time: Res<Time>, mut blacklisted_worker_query: Query<&mut BlacklistedWorkers>) {
    for mut blacklisted_workers in &mut blacklisted_worker_query {
        let mut to_remove = Vec::new();
        for (worker_entity, timer) in blacklisted_workers.0.iter_mut() {
            timer.tick(time.delta());
            if timer.finished() {
                to_remove.push(*worker_entity);
            }
        }
        for worker_entity in to_remove {
            blacklisted_workers.0.remove(&worker_entity);
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
            Option<&MoveTo>,
            &KinematicCharacterControllerOutput,
        ),
        (With<Worker>, With<HasJob>),
    >,
) {
    for (worker_entity, opt_stuck_timer, opt_move_to, controller_output) in &worker_query {
        if opt_move_to.is_none() && opt_stuck_timer.is_some() {
            commands.entity(worker_entity).remove::<StuckTimer>();
            continue;
        }

        let standing_still = controller_output.effective_translation.x == 0.
            && controller_output.effective_translation.y == 0.;
        if !standing_still && opt_stuck_timer.is_some() {
            commands.entity(worker_entity).remove::<StuckTimer>();
        } else if standing_still && opt_stuck_timer.is_none() {
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
    mut blacklisted_workers_query: Query<&mut BlacklistedWorkers, With<Job>>,
) {
    for (worker_entity, mut stuck_timer, has_job) in &mut stuck_timer_query {
        if stuck_timer.0.tick(time.delta()).just_finished() {
            commands.entity(worker_entity).remove::<StuckTimer>();
            commands.entity(worker_entity).remove::<HasJob>();

            commands.entity(has_job.0).remove::<AssignedTo>();
            if let Ok(mut blacklisted_workers) = blacklisted_workers_query.get_mut(has_job.0) {
                blacklisted_workers
                    .0
                    .insert(worker_entity, Timer::from_seconds(5., TimerMode::Once));
            } else {
                let map =
                    HashMap::from([(worker_entity, Timer::from_seconds(5., TimerMode::Once))]);
                commands.entity(has_job.0).insert(BlacklistedWorkers(map));
            }
        }
    }
}
