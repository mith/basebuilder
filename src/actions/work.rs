use std::{fmt::Debug, marker::PhantomData};

use bevy::{
    math::Vec3Swizzles,
    prelude::{
        App, Commands, Component, Entity, GlobalTransform, IntoSystemConfigs, Plugin, PreUpdate,
        Query, With, Without,
    },
};
use big_brain::{
    actions::StepsBuilder,
    prelude::{ActionBuilder, ActionState, FirstToScore, Highest, Measure, ScorerBuilder, Steps},
    scorers::{Score, WinningScorer, WinningScorerBuilder},
    thinker::{ActionSpan, Actor, ScorerSpan, Thinker, ThinkerBuilder},
    BigBrainSet,
};
use tracing::{debug, error, info};

use crate::{
    actions::{action_area::ActionAreaReachable, do_dig_job::do_dig_job, do_fell_job::do_fell_job},
    labor::{
        chop_tree::FellingJob,
        dig_tile::DigJob,
        job::{AssignedJob, AssignedWorker, CanceledJob, Job, JobManagerParams},
    },
};

use super::action_area::{ActionAreaParam, GlobalActionArea};

pub struct WorkPlugin;

impl Plugin for WorkPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            (
                currently_assigned_job,
                currently_assigned_job_type::<FellingJob>,
                currently_assigned_job_type::<DigJob>,
                assigned_job_unreachable::<FellingJob>,
                assigned_job_unreachable::<DigJob>,
            )
                .in_set(BigBrainSet::Scorers),
        )
        .add_systems(
            PreUpdate,
            (
                check_job_canceled,
                pick_job_shortest_path::<FellingJob>,
                pick_job_shortest_path::<DigJob>,
                complete_job,
                cancel_job_assignment,
            )
                .in_set(BigBrainSet::Actions),
        );
    }
}

/// Create a worker thinker builder.
///
/// This thinker builder will create a thinker that can do jobs.
pub fn worker_thinker_builder() -> ThinkerBuilder {
    info!("Building worker thinker");
    Thinker::build()
        .label("worker")
        .picker(Highest)
        .when(AssignedJobUnreachable, CancelJobAssignment)
        .when(
            job_scorer_builder::<FellingJob>(),
            do_job::<FellingJob, _>(do_fell_job()),
        )
        .when(
            job_scorer_builder::<DigJob>(),
            do_job::<DigJob, _>(do_dig_job()),
        )
}

/// Create a job scorer builder.
///
/// This scorer builder will score jobs of type T based on whether they are assigned to the actor or
/// if they are reachable from the actor's current position
fn job_scorer_builder<T>() -> WinningScorerBuilder
where
    T: bevy::prelude::Component + GlobalActionArea + std::fmt::Debug,
{
    let dig_job_assigned_or_available = WinningScorer::build(0.8)
        .push(CurrentlyAssignedJobType::<T>::build())
        .push(ActionAreaReachable::<T, Without<AssignedWorker>>::build());
    dig_job_assigned_or_available
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
        .push(ActionAreaReachable::<FellingJob, Without<AssignedWorker>>::build())
        .push(ActionAreaReachable::<DigJob, Without<AssignedWorker>>::build())
        .push(CurrentlyAssignedJob)
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

#[derive(Component, Debug, Clone, ScorerBuilder)]
pub struct AssignedJobUnreachable;

fn assigned_job_unreachable<T>(
    mut actor_query: Query<(&Actor, &mut Score, &ScorerSpan), With<AssignedJobUnreachable>>,
    assigned_job_query: Query<&AssignedJob>,
    job_query: Query<&T>,
    global_transform_query: Query<&GlobalTransform>,
    action_area_param: ActionAreaParam<T>,
) where
    T: Component + GlobalActionArea + std::fmt::Debug,
{
    for (actor, mut score, _span) in &mut actor_query {
        let Ok(actor_pos) = global_transform_query.get(actor.0).map(|t| t.translation().xy()) else {
            error!("Actor should have a global transform");
            score.set(0.0);
            continue;
        };
        let path_to_job = assigned_job_query
            .get(actor.0)
            .and_then(|assigned_job| job_query.get(assigned_job.0))
            .ok()
            .and_then(|job| action_area_param.path_to_action_area(actor_pos, job));

        if path_to_job.is_none() {
            score.set(1.0);
        } else {
            score.set(0.0);
        }
    }
}

#[derive(Component, Debug, Clone, ActionBuilder)]
pub struct CancelJobAssignment;

fn cancel_job_assignment(
    mut action_query: Query<(&Actor, &mut ActionState, &ActionSpan), With<CancelJobAssignment>>,
    assigned_job_query: Query<&AssignedJob>,
    mut job_manager_params: JobManagerParams,
) {
    for (actor, mut action_state, span) in &mut action_query {
        let _guard = span.span().enter();

        match *action_state {
            ActionState::Requested => {
                info!("Starting cancel job assignment");
                *action_state = ActionState::Executing;
            }
            ActionState::Executing => {
                info!("Canceling job assignment");
                if let Ok(assigned_job) = assigned_job_query.get(actor.0) {
                    info!(job=?assigned_job, "Canceling job assignment");
                    job_manager_params.cancel_job_assignment(assigned_job.0, actor.0);
                    *action_state = ActionState::Success;
                } else {
                    info!("No job to cancel");
                    *action_state = ActionState::Failure;
                }
            }
            ActionState::Cancelled => {
                info!("Canceling job assignment cancelled");
                *action_state = ActionState::Failure;
            }
            _ => {}
        }
    }
}

#[derive(Component, Debug, Clone)]
pub struct CurrentlyAssignedJobType<T: Component>(PhantomData<T>);

impl<T: bevy::prelude::Component> CurrentlyAssignedJobType<T> {
    pub fn build() -> CurrentlyAssignedJobTypeBuilder<T> {
        CurrentlyAssignedJobTypeBuilder(PhantomData)
    }
}

#[derive(Debug, Clone)]
pub struct CurrentlyAssignedJobTypeBuilder<T>(PhantomData<T>)
where
    T: Component;

impl<T> ScorerBuilder for CurrentlyAssignedJobTypeBuilder<T>
where
    T: Component + Debug,
{
    fn build(&self, cmd: &mut Commands, scorer: Entity, _actor: Entity) {
        cmd.entity(scorer)
            .insert(CurrentlyAssignedJobType::<T>(std::marker::PhantomData));
    }
}

fn currently_assigned_job_type<T>(
    mut actor_query: Query<(&Actor, &mut Score, &ScorerSpan), With<CurrentlyAssignedJobType<T>>>,
    assigned_job_query: Query<&AssignedJob>,
    job_query: Query<&T>,
) where
    T: Component,
{
    for (actor, mut score, _span) in &mut actor_query {
        if assigned_job_query
            .get(actor.0)
            .and_then(|assigned_job| job_query.get(assigned_job.0))
            .is_ok()
        {
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

pub fn pick_job_shortest_path<T: Component + GlobalActionArea>(
    mut action_query: Query<(&Actor, &mut ActionState, &ActionSpan), With<PickJob<T>>>,
    job_query: Query<(Entity, &T), Without<AssignedWorker>>,
    global_transform_query: Query<&GlobalTransform>,
    mut job_manager_params: JobManagerParams,
    action_area_param: ActionAreaParam<T>,
) {
    let jobs: Vec<_> = job_query.iter().collect();
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
                let Ok(actor_position) = global_transform_query.get(actor.0).map(|t| t.translation().xy()) else {
                    error!("Actor should have a global transform");
                    *action_state = ActionState::Failure;
                    continue;
                };

                let shortest_path_job = jobs
                    .iter()
                    .flat_map(|(job_entity, job)| {
                        let path = action_area_param.path_to_action_area(actor_position, job)?;
                        Some((path, *job_entity))
                    })
                    .min_by_key(|(path, _)| path.0.len())
                    .map(|(_, job_entity)| job_entity);
                if let Some(job_entity) = shortest_path_job {
                    info!(job=?job_entity, "Picked job with shortest commute");
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
