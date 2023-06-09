use bevy::{
    ecs::system::SystemParam,
    math::Vec3Swizzles,
    prelude::{
        apply_system_buffers, info, Added, App, Commands, Component, DespawnRecursiveExt, Entity,
        EventReader, EventWriter, GlobalTransform, IntoSystemConfigs, Parent, Plugin, Query,
        RemovedComponents, Res, SystemSet, Vec2, With, Without,
    },
    reflect::Reflect,
    time::{Time, Timer},
    utils::{HashMap, HashSet},
};
use tracing::{debug, instrument, warn};

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
            .register_type::<JobSite>()
            .add_systems((find_pathable_workers, blacklist_timer))
            .add_systems(
                (apply_system_buffers, assign_jobs, apply_system_buffers)
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

/// A job assigned to a worker
#[derive(Component, Reflect)]
pub struct AssignedJob(pub Entity);

/// A worker assigned to a job
#[derive(Component, Reflect)]
pub struct AssignedTo(pub Entity);

#[derive(Component, Reflect)]
pub struct BlacklistedWorkers(pub HashMap<Entity, Timer>);

#[derive(Component, Clone, Reflect, Debug)]
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

#[derive(SystemParam)]
pub struct JobManagerParams<'w, 's> {
    commands: Commands<'w, 's>,
    job_assigned_event_writer: EventWriter<'w, JobAssignedEvent>,
}

impl JobManagerParams<'_, '_> {
    pub fn assign_job(&mut self, job_entity: Entity, worker_entity: Entity) {
        self.commands
            .entity(worker_entity)
            .insert(AssignedJob(job_entity));
        self.commands
            .entity(job_entity)
            .insert(AssignedTo(worker_entity));
        self.job_assigned_event_writer.send(JobAssignedEvent {
            job: job_entity,
            worker: worker_entity,
        });
        info!(
            "Assigned job {:?} to worker {:?}",
            job_entity, worker_entity
        );
    }
}

#[instrument(skip(
    unassigned_job_query,
    assigned_job_query,
    worker_query,
    job_manager_params
))]
pub fn assign_jobs(
    unassigned_job_query: Query<
        (
            Entity,
            Option<&BlacklistedWorkers>,
            &PathableWorkers,
            &EligibleWorkers,
            Option<&Parent>,
        ),
        (With<Job>, Without<AssignedTo>),
    >,
    assigned_job_query: Query<&AssignedTo, With<Job>>,
    worker_query: Query<(Entity, &GlobalTransform), (With<Worker>, Without<AssignedJob>)>,
    mut job_manager_params: JobManagerParams,
) {
    let mut unassigned_workers: HashMap<Entity, &GlobalTransform> = HashMap::from_iter(
        worker_query
            .iter()
            .map(|(entity, transform)| (entity, transform)),
    );
    // Look for unnassigned jobs and assign them to the closest unnoccupied worker
    for (job_entity, opt_blacklisted_workers, pathable_workers, eligible_workers, opt_parent) in
        &unassigned_job_query
    {
        if let Some(AssignedTo(worker_entity)) =
            opt_parent.and_then(|parent| assigned_job_query.get(parent.get()).ok())
        {
            // if the job is a subjob, and the parent job is assigned to a worker, assign the subjob to the same worker
            job_manager_params.assign_job(job_entity, *worker_entity);
            continue;
        }
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
                    if eligible_workers.0.contains(worker_entity)
                        && unassigned_workers.contains_key(worker_entity)
                    {
                        Some((*worker_entity, path))
                    } else {
                        None
                    }
                }),
        );

        // find closest available worker through sorting by path length
        let closest_available_worker = available_workers
            .iter()
            .min_by_key(|(_, path)| path.0.len())
            .map(|(worker_entity, _)| (*worker_entity, available_workers[worker_entity]));

        if let Some((worker_entity, path)) = closest_available_worker {
            // assign job to worker
            job_manager_params.assign_job(job_entity, worker_entity);
            // remove worker from available workers
            if unassigned_workers.remove(&worker_entity).is_none() {
                warn!("Worker {:?} was not in unassigned workers", worker_entity);
            }
        } else {
            debug!("No available workers for job {:?}", job_entity);
        }
    }
}

pub struct JobCompletedEvent<T> {
    job_entity: Entity,
    parent_job_entity: Option<Entity>,
    worker_entity: Entity,
    job: T,
}

pub fn register_job<J, W>(app: &mut App)
where
    J: bevy::prelude::Component + std::clone::Clone,
    W: bevy::prelude::Component + Default + core::fmt::Debug,
{
    app.add_event::<JobCompletedEvent<J>>()
        .add_systems((job_assigned::<J, W>, job_completed::<J>));
}

#[instrument(skip(commands, assigned_job_events, job_query))]
fn job_assigned<J, W>(
    mut commands: Commands,
    mut assigned_job_events: EventReader<JobAssignedEvent>,
    job_query: Query<Entity, With<J>>,
) where
    J: Component,
    W: Component + Default + core::fmt::Debug,
{
    for assignment in assigned_job_events.iter() {
        if job_query.get(assignment.job).is_err() {
            // Assigned job is not of type J
            continue;
        }
        info!("Worker {:?} is now a {:?}", assignment.worker, W::default());
        commands.entity(assignment.worker).insert(W::default());
    }
}

#[derive(Component)]
pub struct Complete;

#[instrument(skip(commands, completed_job_query, job_query, job_completed_events))]
fn job_completed<J>(
    mut commands: Commands,
    mut completed_job_query: Query<
        (Entity, &J, &AssignedTo, Option<&Parent>),
        (With<Job>, Added<Complete>),
    >,
    job_query: Query<&Job>,
    mut job_completed_events: EventWriter<JobCompletedEvent<J>>,
) where
    J: std::marker::Send + std::marker::Sync + 'static + bevy::prelude::Component + Clone,
{
    // If the completed job has a parent, assign the worker to the parent
    // If the job has no parent, unassign the worker
    for (job_entity, job, assigned_to, opt_parent) in &mut completed_job_query {
        info!("Job {:?} completed", job_entity);
        job_completed_events.send(JobCompletedEvent {
            job_entity,
            parent_job_entity: opt_parent.map(|p| p.get()),
            worker_entity: assigned_to.0,
            job: job.clone(),
        });
        if let Some(parent) = opt_parent.filter(|p| job_query.contains(p.get())) {
            commands
                .entity(parent.get())
                .insert(AssignedTo(assigned_to.0));

            remove_commute(&mut commands, assigned_to.0);

            info!(
                "Assigned worker {:?} to parent {:?}",
                assigned_to.0,
                parent.get()
            );

            commands
                .entity(assigned_to.0)
                .insert(AssignedJob(parent.get()));
        } else {
            commands.entity(assigned_to.0).remove::<AssignedJob>();
            remove_commute(&mut commands, assigned_to.0);
        }

        commands.entity(job_entity).despawn_recursive();
        info!("Despawned job {:?}", job_entity);
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

pub fn remove_commute(commands: &mut Commands, worker_entity: Entity) {
    info!("Removing commute for worker {:?}", worker_entity);
    commands
        .entity(worker_entity)
        .remove::<Commuting>()
        .remove::<AtJobSite>()
        .remove::<Path>();
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

pub trait WorkerTrait {
    type Worker: Component + Default;

    fn worker() -> Self::Worker {
        Self::Worker::default()
    }
}
