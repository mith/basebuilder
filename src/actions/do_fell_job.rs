use bevy::{
    prelude::{
        App, Commands, Component, Entity, GlobalTransform, IntoSystemConfigs, Plugin, PreUpdate,
        Query, Resource, With,
    },
    utils::HashSet,
};
use big_brain::{
    actions::StepsBuilder,
    prelude::{ActionBuilder, ActionState, ConcurrentMode, Concurrently, ScorerBuilder, Steps},
    scorers::Score,
    thinker::{ActionSpan, Actor, ScorerSpan},
    BigBrainSet,
};
use tracing::{debug, info};

use crate::{
    actions::fell::FellTarget,
    labor::{chop_tree::FellingJob, job::AssignedJob},
    tree::Tree,
};

use super::{
    fell::fell_tree,
    work::{complete_job, pick_job, CheckJobCanceled, CompleteJob, PickJob, UnassignWorker},
};
pub struct DoFellingJobPlugin;

impl Plugin for DoFellingJobPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, (feller_scorer).in_set(BigBrainSet::Scorers))
            .add_systems(
                PreUpdate,
                (
                    pick_job::<PickFellingJob>,
                    set_fell_target,
                    check_tree_exists,
                    complete_job::<CompleteFellingJob>,
                )
                    .in_set(BigBrainSet::Actions),
            );
    }
}

pub fn do_fell_job() -> StepsBuilder {
    info!("Building do_fell_job");
    let fell = Steps::build()
        .label("fell")
        .step(SetFellTarget)
        .step(fell_tree())
        .step(CompleteFellingJob);

    let do_job = Concurrently::build()
        .mode(ConcurrentMode::Race)
        .label("do_job")
        .push(CheckJobCanceled)
        .push(CheckTreeExists)
        .push(fell);

    Steps::build()
        .label("do_fell_job")
        .step(PickFellingJob)
        .step(do_job)
        .step(UnassignWorker)
}

#[derive(Component, Debug, Clone, ActionBuilder)]
struct SetFellTarget;

fn set_fell_target(
    mut commands: Commands,
    mut action_query: Query<(&Actor, &mut ActionState, &ActionSpan), With<SetFellTarget>>,
    assigned_job_query: Query<&AssignedJob>,
    fell_job_query: Query<&FellingJob>,
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
                    .insert(FellTarget(assigned_fell_job.0));
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

#[derive(Component, Clone, Debug, ScorerBuilder)]
pub struct Feller;

impl PickJob for PickFellingJob {
    type Job = FellingJob;
}

#[derive(Component, Debug, Clone, ActionBuilder)]
struct PickFellingJob;

impl CompleteJob for CompleteFellingJob {
    type Job = FellingJob;
}
#[derive(Component, Debug, Clone, ActionBuilder)]
struct CompleteFellingJob;

#[derive(Resource, Debug)]
struct FellingJobs(HashSet<FellingJob>);

fn feller_scorer(
    mut actor_query: Query<(&Actor, &mut Score, &ScorerSpan), With<Feller>>,
    fell_jobs_query: Query<&FellingJob>,
    _global_transform_query: Query<&GlobalTransform>,
) {
    for (_actor, mut score, span) in &mut actor_query {
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

        // for FellingJob { tree } in &fell_jobs_query {
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
struct CheckTreeExists;

fn check_tree_exists(
    mut action_query: Query<(&Actor, &mut ActionState, &ActionSpan), With<CheckTreeExists>>,
    assigned_job_query: Query<&AssignedJob>,
    fell_job_query: Query<&FellingJob>,
    tree_query: Query<&Tree>,
) {
    for (actor, mut action_state, span) in &mut action_query {
        let _guard = span.span().enter();

        match *action_state {
            ActionState::Requested => {
                info!("Checking tree exists");
                *action_state = ActionState::Executing;
            }
            ActionState::Executing => {
                // Check if the target tree still exists
                if let Ok(AssignedJob(job_entity)) = assigned_job_query.get(actor.0) {
                    if let Ok(FellingJob(tree)) = fell_job_query.get(*job_entity) {
                        if tree_query.get(*tree).is_err() {
                            info!("Tree does not exist");
                            *action_state = ActionState::Success;
                        } else {
                            debug!("Tree still exists");
                        }
                    } else {
                        info!("No fell job");
                        *action_state = ActionState::Cancelled;
                    }
                } else {
                    info!("No assigned job");
                    *action_state = ActionState::Cancelled;
                }
            }
            ActionState::Cancelled => {
                info!("Check tree exists cancelled");
                *action_state = ActionState::Failure;
            }
            _ => {}
        }
    }
}
