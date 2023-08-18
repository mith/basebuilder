use std::fmt::Debug;

use bevy::{
    prelude::{
        App, Commands, Component, Entity, GlobalTransform, IntoSystemConfigs, Plugin, PreUpdate,
        Query, Resource, With, Without,
    },
    utils::HashSet,
};
use big_brain::{
    actions::StepsBuilder,
    prelude::{
        ActionBuilder, ActionState, ConcurrentMode, Concurrently, FirstToScore, ScorerBuilder,
        Steps,
    },
    scorers::Score,
    thinker::{ActionSpan, Actor, ScorerSpan, Thinker, ThinkerBuilder},
    BigBrainSet,
};
use tracing::{debug, info};

use crate::{
    actions::{
        do_fell_job::{do_fell_job, Feller},
        fell::FellTarget,
    },
    labor::job::{AssignedJob, AssignedWorker, Job, JobManagerParams},
    tree::Tree,
};

use super::fell::fell_tree;

pub struct WorkPlugin;

impl Plugin for WorkPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, (jobs_available).in_set(BigBrainSet::Scorers))
            .add_systems(
                PreUpdate,
                (check_job_canceled, unassign_worker).in_set(BigBrainSet::Actions),
            );
    }
}

pub fn build_worker_thinker() -> ThinkerBuilder {
    info!("Building worker thinker");
    Thinker::build()
        .label("worker")
        .picker(FirstToScore::new(0.8))
        .when(Feller, do_fell_job())
}

#[derive(Component, Debug, Clone, ScorerBuilder)]
pub struct JobsAvailable;

fn jobs_available(
    mut actor_query: Query<(&Actor, &mut Score, &ScorerSpan), With<JobsAvailable>>,
    unassigned_jobs_query: Query<Entity, (With<Job>, Without<AssignedWorker>)>,
    assigned_job_query: Query<&AssignedJob>,
) {
    let any_jobs_available = unassigned_jobs_query.iter().next().is_some();
    for (actor, mut score, _span) in &mut actor_query {
        // Check if the actor is currently assigned a job or if there are unassigned jobs availabe
        let currently_assigned_job = assigned_job_query.get(actor.0).is_ok();
        if any_jobs_available || currently_assigned_job {
            score.set(1.0);
        } else {
            score.set(0.0);
        }
    }
}

pub trait PickJob {
    type Job: Component;
}

pub fn pick_job<TPickJobAction: std::fmt::Debug + Component + PickJob>(
    mut action_query: Query<(&Actor, &mut ActionState, &ActionSpan), With<TPickJobAction>>,
    job_query: Query<Entity, (With<TPickJobAction::Job>, Without<AssignedWorker>)>,
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

pub trait CompleteJob {
    type Job: Component;
}

pub fn complete_job<TCompleteJobAction: std::fmt::Debug + Component + CompleteJob>(
    mut action_query: Query<(&Actor, &mut ActionState, &ActionSpan), With<TCompleteJobAction>>,
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
                if assigned_job_query.get(actor.0).is_err() {
                    info!("Job canceled");
                    *action_state = ActionState::Cancelled;
                } else {
                    debug!("Job not canceled");
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

#[derive(Component, Debug, Clone, ActionBuilder)]
pub struct UnassignWorker;

fn unassign_worker(
    mut commands: Commands,
    mut action_query: Query<(&Actor, &mut ActionState, &ActionSpan), With<UnassignWorker>>,
) {
    for (actor, mut action_state, span) in &mut action_query {
        let _guard = span.span().enter();

        match *action_state {
            ActionState::Requested => {
                info!("Unassigning worker");
                *action_state = ActionState::Executing;
            }
            ActionState::Executing => {
                commands.entity(actor.0).remove::<AssignedJob>();
                *action_state = ActionState::Success;
            }
            ActionState::Cancelled => {
                info!("Unassign worker cancelled");
                *action_state = ActionState::Failure;
            }
            _ => {}
        }
    }
}
