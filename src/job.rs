use bevy::{
    math::Vec3Swizzles,
    prelude::*,
    utils::{HashMap, HashSet},
};
use bevy_ecs_tilemap::tiles::TilePos;
use bevy_rapier2d::prelude::KinematicCharacterControllerOutput;

use crate::{
    ai_controller::{MoveTo, Path},
    pathfinding::Pathfinding,
};

pub struct JobPlugin;

impl Plugin for JobPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<JobAssignedEvent>()
            .register_type::<AssignedJob>()
            .register_type::<AssignedTo>()
            .register_type::<BlacklistedWorkers>()
            .register_type::<PathableWorkers>()
            .register_type::<EligibleWorkers>()
            .add_systems((
                find_pathable_workers,
                assign_job,
                blacklist_timer,
                stuck,
                stuck_timer,
            ))
            .add_systems(
                (
                    apply_system_buffers,
                    job_unassigned,
                    apply_system_buffers,
                    commute,
                )
                    .chain(),
            );
    }
}

#[derive(Component)]
pub struct Job;

#[derive(Component)]
pub struct Worker;

#[derive(Component, Reflect)]
pub struct AssignedJob(pub Entity);

#[derive(Component, Reflect)]
pub struct AssignedTo(pub Entity);

#[derive(Component, Reflect)]
pub struct BlacklistedWorkers(HashMap<Entity, Timer>);

#[derive(Component, Clone)]
pub struct JobSite(pub Vec<Vec2>);

#[derive(Component)]
pub struct Commuting;

#[derive(Component)]
pub struct AtJobSite;

pub struct JobAssignedEvent {
    pub job: Entity,
    pub worker: Entity,
}

#[derive(Component, Reflect)]
pub struct PathableWorkers(pub HashMap<Entity, Path>);

pub fn find_pathable_workers(
    mut commands: Commands,
    unassigned_job_query: Query<(Entity, &JobSite), (With<Job>, Without<AssignedTo>)>,
    worker_query: Query<(Entity, &GlobalTransform), (With<Worker>, Without<AssignedJob>)>,
    pathfinder: Pathfinding,
) {
    let unassigned_workers = worker_query.iter().collect::<Vec<_>>();
    // Look for unnassigned jobs and assign them to the closest unnoccupied worker
    for (job_entity, job_site) in &unassigned_job_query {
        // find path for each worker, discard worker if no path found
        let pathable_workers: HashMap<Entity, Path> = unassigned_workers
            .iter()
            .filter_map(|(worker_entity, worker_transform)| {
                let pathable_job_sites = job_site.0.iter().filter_map(|site_pos| {
                    pathfinder.find_path(worker_transform.translation().xy(), *site_pos)
                });

                if let Some(path) = pathable_job_sites.min_by_key(|path| path.len()) {
                    Some((*worker_entity, Path(path)))
                } else {
                    None
                }
            })
            .collect();

        commands
            .entity(job_entity)
            .insert(PathableWorkers(pathable_workers));
    }
}

#[derive(Component, Reflect)]
pub struct EligibleWorkers(pub HashSet<Entity>);

pub fn assign_job(
    mut commands: Commands,
    unassigned_job_query: Query<
        (
            Entity,
            Option<&BlacklistedWorkers>,
            &PathableWorkers,
            &EligibleWorkers,
        ),
        (With<Job>, Without<AssignedTo>),
    >,
    worker_query: Query<(Entity, &GlobalTransform), (With<Worker>, Without<AssignedJob>)>,
) {
    let mut unassigned_workers: HashMap<Entity, &GlobalTransform> = HashMap::from_iter(
        worker_query
            .iter()
            .map(|(entity, transform)| (entity, transform)),
    );
    // Look for unnassigned jobs and assign them to the closest unnoccupied worker
    for (job_entity, opt_blacklisted_workers, pathable_workers, eligible_workers) in
        &unassigned_job_query
    {
        // take the intersection of available workers and eligible workers and remove blacklisted workers
        let available_workers: HashMap<Entity, &Path> = HashMap::from_iter(
            pathable_workers
                .0
                .iter()
                .filter_map(|(worker_entity, path)| {
                    if let Some(blacklisted_workers) = opt_blacklisted_workers {
                        if blacklisted_workers.0.contains_key(worker_entity) {
                            return None;
                        }
                    }
                    if eligible_workers.0.contains(worker_entity) {
                        Some((*worker_entity, path))
                    } else {
                        None
                    }
                }),
        );

        // find closest available worker
        let closest_available_worker = available_workers
            .iter()
            .min_by_key(|(_, path)| path.0.len())
            .map(|(worker_entity, _)| (*worker_entity, available_workers[worker_entity]));

        if let Some((worker_entity, path)) = closest_available_worker {
            // assign job to worker
            commands.entity(worker_entity).insert((
                AssignedJob(job_entity),
                Commuting,
                path.clone(),
            ));
            commands
                .entity(job_entity)
                .insert(AssignedTo(worker_entity));
            // remove worker from available workers
            unassigned_workers.remove(&worker_entity);
        }
    }
}

pub fn job_assigned<J, W>(
    mut commands: Commands,
    assigned_job_query: Query<&AssignedTo, (With<J>, Added<AssignedTo>)>,
) where
    J: Component,
    W: Component + Default,
{
    for assigned_to in &assigned_job_query {
        commands.entity(assigned_to.0).insert(W::default());
    }
}

pub fn all_workers_eligible<J>(
    mut commands: Commands,
    new_job_query: Query<Entity, (With<J>, Without<EligibleWorkers>, Added<Job>)>,
    worker_query: Query<Entity, With<Worker>>,
) where
    J: Component,
{
    for job in &new_job_query {
        commands
            .entity(job)
            .insert(EligibleWorkers(HashSet::from_iter(worker_query.iter())));
    }
}

fn commute(
    mut commands: Commands,
    workers_query: Query<
        (Entity, &AssignedJob, &GlobalTransform, Option<&Path>),
        Without<AtJobSite>,
    >,
    job_query: Query<&JobSite>,
    pathfinder: Pathfinding,
) {
    for (worker_entity, assigned_job, worker_transform, opt_path) in &workers_query {
        // if the worker already has a path, add commute component and continue
        if opt_path.is_some() {
            continue;
        }

        let job_site = job_query
            .get(assigned_job.0)
            .expect("Worker has job without job site");

        // check if worker is already at job site
        if job_site.0.iter().any(|job_site_world_pos| {
            worker_transform
                .translation()
                .xy()
                .distance(*job_site_world_pos)
                < 10.
        }) {
            commands
                .entity(worker_entity)
                .insert(AtJobSite)
                .remove::<Commuting>();
            continue;
        }

        // find job site tile with the shortest path from worker position
        let paths = job_site.0.iter().filter_map(|job_site_world_pos| {
            pathfinder.find_path(worker_transform.translation().xy(), *job_site_world_pos)
        });
        let shortest_path = paths.min_by(|path_a, path_b| path_a.len().cmp(&path_b.len()));

        if let Some(path) = shortest_path {
            commands.entity(worker_entity).insert(Commuting);
            commands.entity(worker_entity).insert(Path(path));
        } else {
            // no path found, start stucktimer
            commands.entity(worker_entity).insert(StuckTimer::default());
        }
    }
}

fn job_unassigned(
    mut commands: Commands,
    mut removed_assigned_job: RemovedComponents<AssignedJob>,
) {
    for unassigned_worker_entity in removed_assigned_job.iter() {
        unassign_job(&mut commands, unassigned_worker_entity);
    }
}

pub fn unassign_job(commands: &mut Commands, unassigned_worker_entity: Entity) {
    commands
        .entity(unassigned_worker_entity)
        .remove::<Commuting>()
        .remove::<JobSite>()
        .remove::<AtJobSite>();
    commands.entity(unassigned_worker_entity).remove::<Path>();
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
pub struct StuckTimer(Timer);

impl Default for StuckTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(1., TimerMode::Once))
    }
}

fn stuck(
    mut commands: Commands,
    worker_query: Query<
        (
            Entity,
            Option<&StuckTimer>,
            Option<&MoveTo>,
            &KinematicCharacterControllerOutput,
        ),
        (With<Worker>, With<AssignedJob>),
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
    mut stuck_timer_query: Query<(Entity, &mut StuckTimer, &AssignedJob), With<Worker>>,
    mut blacklisted_workers_query: Query<&mut BlacklistedWorkers, With<Job>>,
) {
    for (worker_entity, mut stuck_timer, assigned_job) in &mut stuck_timer_query {
        if stuck_timer.0.tick(time.delta()).just_finished() {
            commands.entity(worker_entity).remove::<StuckTimer>();
            commands.entity(worker_entity).remove::<AssignedJob>();

            commands.entity(assigned_job.0).remove::<AssignedTo>();
            if let Ok(mut blacklisted_workers) = blacklisted_workers_query.get_mut(assigned_job.0) {
                blacklisted_workers
                    .0
                    .insert(worker_entity, Timer::from_seconds(5., TimerMode::Once));
            } else {
                let map =
                    HashMap::from([(worker_entity, Timer::from_seconds(5., TimerMode::Once))]);
                commands
                    .entity(assigned_job.0)
                    .insert(BlacklistedWorkers(map));
            }
        }
    }
}
