use bevy::{
    ecs::system::SystemParam,
    prelude::{
        Added, App, Commands, Component, Entity, Event, EventWriter, Plugin, Query, SystemSet,
        Vec2, With, Without,
    },
    reflect::Reflect,
    time::Timer,
    utils::{HashMap, HashSet},
};
use tracing::info;

pub struct JobPlugin;

impl Plugin for JobPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<JobAssignedEvent>()
            .add_event::<JobCompletedEvent>()
            .register_type::<AssignedJob>()
            .register_type::<AssignedWorker>()
            .register_type::<BlacklistedWorkers>()
            .register_type::<EligibleWorkers>()
            .register_type::<JobSite>()
            .register_type::<JobAssignedEvent>()
            .register_type::<JobCompletedEvent>();
    }
}

#[derive(SystemSet, Default, Debug, Hash, PartialEq, Eq, Clone)]
pub struct JobAssignmentSet;

#[derive(Component)]
pub struct Job;

#[derive(Component)]
pub struct Worker;

/// A job assigned to a worker
#[derive(Component, Debug, Reflect)]
pub struct AssignedJob(pub Entity);

// A worker assigned to a job
#[derive(Component, Debug, Reflect)]
pub struct AssignedWorker(pub Entity);

#[derive(Component, Debug, Reflect)]
pub struct BlacklistedWorkers(pub HashMap<Entity, Timer>);

#[derive(Component, Clone, Reflect, Debug)]
pub struct JobSite(pub Vec<Vec2>);

#[derive(Event, Debug, Reflect)]
pub struct JobAssignedEvent {
    pub job: Entity,
    pub worker: Entity,
}

#[derive(Event, Debug, Reflect)]
pub struct JobCompletedEvent {
    job_entity: Entity,
    worker_entity: Entity,
}

#[derive(Component)]
pub struct CompletedJob;

#[derive(Component)]
pub struct CanceledJob;

#[derive(Component, Debug, Reflect)]
pub struct EligibleWorkers(pub HashSet<Entity>);

#[derive(SystemParam)]
pub struct JobManagerParams<'w, 's> {
    commands: Commands<'w, 's>,
    job_assigned_event_writer: EventWriter<'w, JobAssignedEvent>,
    job_completed_events: EventWriter<'w, JobCompletedEvent>,
}

impl JobManagerParams<'_, '_> {
    pub fn assign_job(&mut self, job_entity: Entity, worker_entity: Entity) {
        self.commands
            .entity(worker_entity)
            .insert(AssignedJob(job_entity));
        self.commands
            .entity(job_entity)
            .insert(AssignedWorker(worker_entity));
        self.job_assigned_event_writer.send(JobAssignedEvent {
            job: job_entity,
            worker: worker_entity,
        });
        info!(
            "Assigned job {:?} to worker {:?}",
            job_entity, worker_entity
        );
    }

    pub fn complete_job(&mut self, job_entity: Entity, worker_entity: Entity) {
        self.commands
            .entity(job_entity)
            .remove::<Job>()
            .insert(CompletedJob);

        self.job_completed_events.send(JobCompletedEvent {
            job_entity,
            worker_entity,
        });
    }

    pub fn cancel_job(&mut self, job_entity: Entity) {
        self.commands
            .entity(job_entity)
            .remove::<Job>()
            .insert(CanceledJob);
    }
}

pub fn all_workers_eligible<JobType>(
    mut commands: Commands,
    new_job_query: Query<Entity, (With<JobType>, Without<EligibleWorkers>, Added<Job>)>,
    worker_query: Query<Entity, With<Worker>>,
) where
    JobType: Component,
{
    for job in &new_job_query {
        commands
            .entity(job)
            .insert(EligibleWorkers(HashSet::from_iter(worker_query.iter())));
    }
}
