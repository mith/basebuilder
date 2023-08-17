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
use tracing::{debug, error, info};

use crate::{
    actions::fell::FellTarget,
    labor::job::{AssignedJob, AssignedWorker, Job, JobManagerParams},
};

use super::fell::fell_tree;

pub struct WorkPlugin;

impl Plugin for WorkPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            (jobs_available, feller_scorer).in_set(BigBrainSet::Scorers),
        )
        .add_systems(
            PreUpdate,
            (
                pick_job::<PickFellJob>,
                check_job_canceled,
                set_fell_target,
                remove_fell_target,
                unassign_worker,
            )
                .in_set(BigBrainSet::Actions),
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
    for (actor, mut score, span) in &mut actor_query {
        // Check if the actor is currently assigned a job or if there are unassigned jobs availabe
        let currently_assigned_job = assigned_job_query.get(actor.0).is_ok();
        if any_jobs_available || currently_assigned_job {
            score.set(1.0);
        } else {
            score.set(0.0);
        }
    }
}

trait PickJob {
    type Job: Component;
}

impl PickJob for PickFellJob {
    type Job = FellJob;
}

#[derive(Component, Debug, Clone, ActionBuilder)]
struct PickFellJob;

fn pick_job<TPickJobAction: std::fmt::Debug + Component + PickJob>(
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

#[derive(Component, Debug, Clone, ActionBuilder)]
struct SetFellTarget;

fn set_fell_target(
    mut commands: Commands,
    mut action_query: Query<(&Actor, &mut ActionState, &ActionSpan), With<SetFellTarget>>,
    assigned_job_query: Query<&AssignedJob>,
    fell_job_query: Query<&FellJob>,
) {
    for (actor, mut action_state, span) in &mut action_query {
        let _guard = span.span().enter();

        match *action_state {
            ActionState::Requested => {
                info!("Setting fell target");
                *action_state = ActionState::Executing;
            }
            ActionState::Executing => {
                // Add a FellTarget component to the actor
                let assigned_fell_job = assigned_job_query
                    .get(actor.0)
                    .and_then(|assigned_job| fell_job_query.get(assigned_job.0))
                    .expect("Actor should have an assigned job");

                info!(job=?assigned_fell_job, "Setting fell target");
                commands
                    .entity(actor.0)
                    .insert(FellTarget(assigned_fell_job.tree));
                *action_state = ActionState::Success;
            }
            ActionState::Cancelled => {
                info!("Setting fell target cancelled");
                *action_state = ActionState::Failure;
            }
            _ => {}
        }
    }
}

#[derive(Component, Debug, Clone, ActionBuilder)]
struct RemoveFellTarget;

fn remove_fell_target(
    mut commands: Commands,
    mut action_query: Query<(&Actor, &mut ActionState, &ActionSpan), With<RemoveFellTarget>>,
) {
    for (actor, mut action_state, span) in &mut action_query {
        let _guard = span.span().enter();

        match *action_state {
            ActionState::Requested => {
                info!("Removing fell target");
                *action_state = ActionState::Executing;
            }
            ActionState::Executing => {
                // Remove the FellTarget component from the actor
                info!("Removing fell target");
                commands.entity(actor.0).remove::<FellTarget>();
                *action_state = ActionState::Success;
            }
            ActionState::Cancelled => {
                info!("Removing fell target cancelled");
                *action_state = ActionState::Failure;
            }
            _ => {}
        }
    }
}

#[derive(Component, Clone, Debug, ScorerBuilder)]
struct Feller;

#[derive(Component, Debug)]
pub struct FellJob {
    pub tree: Entity,
}

#[derive(Resource, Debug)]
struct FellJobs(HashSet<FellJob>);

fn feller_scorer(
    mut actor_query: Query<(&Actor, &mut Score, &ScorerSpan), With<Feller>>,
    fell_jobs_query: Query<&FellJob>,
    global_transform_query: Query<&GlobalTransform>,
) {
    for (actor, mut score, span) in &mut actor_query {
        let _guard = span.span().enter();
        // for now, just return a score of 1.0 when there is a job
        if fell_jobs_query.iter().next().is_some() {
            score.set(1.0);
        } else {
            score.set(0.0);
        }

        // let actor_position = global_transform_query
        //     .get(actor.0)
        //     .unwrap()
        //     .translation()
        //     .xy();

        // let mut scores = vec![];

        // for FellJob { tree } in &fell_jobs_query {
        //     let tree_position = global_transform_query
        //         .get(*tree)
        //         .unwrap()
        //         .translation()
        //         .xy();

        //     let distance = actor_position.distance(tree_position);

        //     const MAX_DISTANCE: f32 = 128.0;
        //     const MIN_DISTANCE: f32 = 16.0;
        //     const SCALE: f32 = 1.0 / (MAX_DISTANCE - MIN_DISTANCE);

        //     // Score on a curve, ranging from 1.0 at 16 and closer distance to 0.0 at 128 and up distance
        //     // distance can range from zero to infinity, but we only care about 16 to 128
        //     let score_value =
        //         1.0 - (distance.clamp(MIN_DISTANCE, MAX_DISTANCE) - MIN_DISTANCE) * SCALE;

        //     info!("Score: {}", score_value);
        //     scores.push(score_value);
        // }

        // // pick highest score
        // let highest_score = scores.into_iter().fold(0.0, f32::max);
        // score.set(highest_score);
    }
}

#[derive(Component, Debug, Clone, ActionBuilder)]
struct CheckJobCanceled;

fn check_job_canceled(
    mut action_query: Query<(&Actor, &mut ActionState, &ActionSpan), With<CheckJobCanceled>>,
    assigned_job_query: Query<&AssignedJob>,
) {
    for (actor, mut action_state, span) in &mut action_query {
        let _guard = span.span().enter();

        match *action_state {
            ActionState::Requested => {
                info!("Checking job canceled");
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
                info!("Check job canceled cancelled");
                *action_state = ActionState::Failure;
            }
            _ => {}
        }
    }
}

#[derive(Component, Debug, Clone, ActionBuilder)]
struct UnassignWorker;

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

fn do_fell_job() -> StepsBuilder {
    let fell = Steps::build()
        .label("fell")
        .step(SetFellTarget)
        .step(fell_tree())
        .step(RemoveFellTarget);

    let do_job = Concurrently::build()
        .mode(ConcurrentMode::Race)
        .label("do_job")
        .push(CheckJobCanceled)
        .push(fell);

    Steps::build()
        .label("do_fell_job")
        .step(PickFellJob)
        .step(do_job)
        .step(UnassignWorker)
}
