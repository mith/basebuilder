use bevy::{
    math::Vec3Swizzles,
    prelude::{
        apply_system_buffers, Added, App, Commands, Component, DespawnRecursiveExt, Entity,
        GlobalTransform, IntoSystemConfigs, Parent, Plugin, Query, RemovedComponents, Res,
        SystemSet, Vec2, With, Without,
    },
    reflect::Reflect,
    time::{Time, Timer},
    utils::{HashMap, HashSet},
};

use crate::{ai_controller::Path, pathfinding::Pathfinding};

use super::commute::Commuting;

pub struct JobPlugin;

impl Plugin for JobPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<JobAssignedEvent>()
            .register_type::<AssignedJob>()
            .register_type::<AssignedTo>()
            .register_type::<BlacklistedWorkers>()
            .register_type::<PathableWorkers>()
            .register_type::<EligibleWorkers>()
            .add_systems((find_pathable_workers, blacklist_timer))
            .add_systems(
                (
                    apply_system_buffers,
                    assign_jobs,
                    apply_system_buffers,
                    job_completed,
                    apply_system_buffers,
                )
                    .chain()
                    .in_set(JobAssignmentSet),
            )
            .add_systems(
                (
                    apply_system_buffers,
                    worker_assignment_removed,
                    apply_system_buffers,
                )
                    .chain()
                    .in_set(JobAssignmentSet),
            );
    }
}

#[derive(SystemSet, Default, Debug, Hash, PartialEq, Eq, Clone)]
pub struct JobAssignmentSet;

#[derive(Component)]
pub struct Job;

#[derive(Component)]
pub struct Worker;

#[derive(Component, Reflect)]
pub struct AssignedJob(pub Entity);

#[derive(Component, Reflect)]
pub struct AssignedTo(pub Entity);

#[derive(Component, Reflect)]
pub struct BlacklistedWorkers(pub HashMap<Entity, Timer>);

#[derive(Component, Clone)]
pub struct JobSite(pub Vec<Vec2>);

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

pub fn assign_jobs(
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
            commands
                .entity(worker_entity)
                .insert((AssignedJob(job_entity), path.clone()));
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
fn worker_assignment_removed(
    mut commands: Commands,
    mut removed_assigned_job: RemovedComponents<AssignedJob>,
) {
    for unassigned_worker_entity in removed_assigned_job.iter() {
        remove_commute(&mut commands, unassigned_worker_entity);
    }
}

pub fn remove_commute(commands: &mut Commands, worker_entity: Entity) {
    commands
        .entity(worker_entity)
        .remove::<Commuting>()
        .remove::<AtJobSite>()
        .remove::<Path>();
}

pub fn assign_job(commands: &mut Commands, worker_entity: Entity, job_entity: Entity) {
    commands
        .entity(worker_entity)
        .insert(AssignedJob(job_entity));
    commands
        .entity(job_entity)
        .insert(AssignedTo(worker_entity));
}

#[derive(Component)]
pub struct Complete;

fn job_completed(
    mut commands: Commands,
    mut completed_job_query: Query<
        (Entity, &AssignedTo, Option<&Parent>),
        (With<Job>, Added<Complete>),
    >,
    job_query: Query<&Job>,
) {
    // If the completed job has a parent, assign the worker to the parent
    // If the job has no parent, unassign the worker
    for (job_entity, assigned_to, opt_parent) in &mut completed_job_query {
        if let Some(parent) = opt_parent.filter(|p| job_query.contains(p.get())) {
            commands
                .entity(parent.get())
                .insert(AssignedTo(assigned_to.0));

            remove_commute(&mut commands, assigned_to.0);

            commands
                .entity(assigned_to.0)
                .insert(AssignedJob(parent.get()));
        } else {
            commands.entity(assigned_to.0).remove::<AssignedJob>();
            remove_commute(&mut commands, assigned_to.0);
        }

        commands.entity(job_entity).despawn_recursive();
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
