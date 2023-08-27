use std::{fmt::Debug, marker::PhantomData};

use bevy::prelude::{
    App, Commands, Component, Entity, IntoSystemConfigs, Plugin, PreUpdate, Query, With, Without,
};
use big_brain::{
    actions::StepsBuilder,
    prelude::{ActionBuilder, ActionState, FirstToScore, ScorerBuilder, Steps},
    scorers::{Score, WinningScorer, WinningScorerBuilder},
    thinker::{ActionSpan, Actor, ScorerSpan, Thinker, ThinkerBuilder},
    BigBrainSet,
};
use tracing::info;

use crate::{
    actions::{action_area::ActionAreaReachable, do_dig_job::do_dig_job, do_fell_job::do_fell_job},
    labor::{
        chop_tree::FellingJob,
        dig_tile::DigJob,
        job::{AssignedJob, AssignedWorker, CanceledJob, Job, JobManagerParams},
    },
};

pub struct WorkPlugin;

impl Plugin for WorkPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            (
                jobs_available,
                job_type_available::<DigJob>,
                job_type_available::<FellingJob>,
                currently_assigned_job,
            )
                .in_set(BigBrainSet::Scorers),
        )
        .add_systems(
            PreUpdate,
            (
                check_job_canceled,
                pick_job::<FellingJob>,
                pick_job::<DigJob>,
                complete_job,
            )
                .in_set(BigBrainSet::Actions),
        );
    }
}

pub fn worker_thinker_builder() -> ThinkerBuilder {
    info!("Building worker thinker");
    Thinker::build()
        .label("worker")
        .picker(FirstToScore::new(0.8))
        // .when(AssignedJobUnreachable, CancelJobAssignment)
        .when(
            ActionAreaReachable::<FellingJob>::build(),
            do_job::<FellingJob, _>(do_fell_job()),
        )
        .when(
            ActionAreaReachable::<DigJob>::build(),
            do_job::<DigJob, _>(do_dig_job()),
        )
}

fn do_job<T: Component + Debug, A: ActionBuilder + 'static>(job_steps: A) -> StepsBuilder {
    info!("Building do_job action");
    Steps::build()
        .label("do_job")
        .step(PickJobBuilder::<T>(PhantomData))
        .step(job_steps)
        .step(CompleteJob)
}

pub fn worker_scorer_builder() -> WinningScorerBuilder {
    info!("Building worker scorer");
    WinningScorer::build(0.8)
        .label("worker")
        .push(JobsAvailable)
        .push(CurrentlyAssignedJob)
}

#[derive(Component, Debug, Clone, ScorerBuilder)]
pub struct JobsAvailable;

fn jobs_available(
    mut actor_query: Query<(&Actor, &mut Score, &ScorerSpan), With<JobsAvailable>>,
    unassigned_jobs_query: Query<Entity, (With<Job>, Without<AssignedWorker>)>,
) {
    let any_jobs_available = !unassigned_jobs_query.is_empty();
    for (_actor, mut score, _span) in &mut actor_query {
        if any_jobs_available {
            score.set(1.0);
        } else {
            score.set(0.0);
        }
    }
}

#[derive(Component, Debug, Clone, ScorerBuilder)]
pub struct CurrentlyAssignedJob;

fn currently_assigned_job(
    mut actor_query: Query<(&Actor, &mut Score, &ScorerSpan), With<CurrentlyAssignedJob>>,
    assigned_job_query: Query<&AssignedJob>,
) {
    for (actor, mut score, _span) in &mut actor_query {
        if assigned_job_query.contains(actor.0) {
            score.set(1.0);
        } else {
            score.set(0.0);
        }
    }
}

fn job_type_available<T>(
    mut actor_query: Query<(&Actor, &mut Score, &ScorerSpan), With<JobsAvailable>>,
    job_query: Query<Entity, (With<T>, Without<AssignedWorker>)>,
) where
    T: Component,
{
    for (_actor, mut score, _span) in &mut actor_query {
        if !job_query.is_empty() {
            score.set(1.0);
        } else {
            score.set(0.0);
        }
    }
}

#[derive(Debug, Clone)]
pub struct PickJobBuilder<T>(std::marker::PhantomData<T>)
where
    T: Component;

impl<T> ActionBuilder for PickJobBuilder<T>
where
    T: Component + Debug,
{
    fn build(&self, cmd: &mut Commands, action: Entity, _actor: Entity) {
        cmd.entity(action)
            .insert(PickJob::<T>(std::marker::PhantomData));
    }
}

#[derive(Component, Debug, Clone)]
pub struct PickJob<T: Component>(std::marker::PhantomData<T>);

pub fn pick_job<T: Component>(
    mut action_query: Query<(&Actor, &mut ActionState, &ActionSpan), With<PickJob<T>>>,
    job_query: Query<Entity, (With<T>, Without<AssignedWorker>)>,
    mut job_manager_params: JobManagerParams,
) {
    let mut jobs = job_query.iter().collect::<Vec<_>>();
    if jobs.is_empty() {
        // No jobs to pick from
        return;
    }

    for (actor, mut action_state, span) in &mut action_query {
        let _guard = span.span().enter();

        match *action_state {
            ActionState::Requested => {
                info!("Starting picking job");
                *action_state = ActionState::Executing;
            }
            ActionState::Executing => {
                info!("Picking job");
                if let Some(job_entity) = jobs.pop() {
                    info!(job=?job_entity, "Picked job");
                    job_manager_params.assign_job(job_entity, actor.0);
                    *action_state = ActionState::Success;
                } else {
                    info!("No jobs left to pick");
                    *action_state = ActionState::Failure;
                }
            }
            ActionState::Cancelled => {
                info!("Pickup cancelled");
                *action_state = ActionState::Failure;
            }
            _ => {}
        }
    }
}

#[derive(Component, Debug, Clone, ActionBuilder)]
pub struct CompleteJob;

pub fn complete_job(
    mut commands: Commands,
    mut action_query: Query<(&Actor, &mut ActionState, &ActionSpan), With<CompleteJob>>,
    assigned_job_query: Query<&AssignedJob>,
    mut job_manager_params: JobManagerParams,
) {
    for (actor, mut action_state, span) in &mut action_query {
        let _guard = span.span().enter();

        match *action_state {
            ActionState::Requested => {
                info!("Starting completing job");
                *action_state = ActionState::Executing;
            }
            ActionState::Executing => {
                info!("Completing job");
                if let Ok(assigned_job) = assigned_job_query.get(actor.0) {
                    info!(job=?assigned_job, "Completing job");
                    job_manager_params.complete_job(assigned_job.0, actor.0);
                    commands.entity(actor.0).remove::<AssignedJob>();
                    *action_state = ActionState::Success;
                } else {
                    info!("No job to complete");
                    *action_state = ActionState::Failure;
                }
            }
            ActionState::Cancelled => {
                info!("Completing job cancelled");
                *action_state = ActionState::Failure;
            }
            _ => {}
        }
    }
}

#[derive(Component, Debug, Clone, ActionBuilder)]
pub struct CheckJobCanceled;

fn check_job_canceled(
    mut action_query: Query<(&Actor, &mut ActionState, &ActionSpan), With<CheckJobCanceled>>,
    assigned_job_query: Query<&AssignedJob>,
    canceled_jobs_query: Query<&CanceledJob>,
) {
    for (actor, mut action_state, span) in &mut action_query {
        let _guard = span.span().enter();

        match *action_state {
            ActionState::Requested => {
                info!("Checking if job is cancelled");
                *action_state = ActionState::Executing;
            }
            ActionState::Executing => {
                // Check if the job has been canceled
                if let Ok(AssignedJob(job_entity)) = assigned_job_query.get(actor.0) {
                    if canceled_jobs_query.get(*job_entity).is_ok() {
                        info!("Job is canceled");
                        *action_state = ActionState::Success;
                    }
                } else {
                    *action_state = ActionState::Cancelled;
                }
            }
            ActionState::Cancelled => {
                info!("Checking if job is canceled canceled");
                *action_state = ActionState::Failure;
            }
            _ => {}
        }
    }
}
