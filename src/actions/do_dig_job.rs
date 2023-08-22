use bevy::{
    prelude::{
        App, Commands, Component, GlobalTransform, IntoSystemConfigs, Plugin, PreUpdate, Query,
        Resource, With,
    },
    utils::HashSet,
};
use bevy_ecs_tilemap::tiles::TilePos;
use big_brain::{
    actions::ConcurrentlyBuilder,
    prelude::{ActionBuilder, ActionState, ConcurrentMode, Concurrently, ScorerBuilder, Steps},
    scorers::Score,
    thinker::{ActionSpan, Actor, ScorerSpan},
    BigBrainSet,
};
use tracing::{debug, info};

use crate::{
    actions::dig::DigTarget,
    labor::{dig_tile::DigJob, job::AssignedJob},
};

use super::dig::dig_tile;
pub struct DoDigJobPlugin;

impl Plugin for DoDigJobPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, (digger_scorer).in_set(BigBrainSet::Scorers))
            .add_systems(
                PreUpdate,
                (set_dig_target, check_tile_exists).in_set(BigBrainSet::Actions),
            );
    }
}

pub fn do_dig_job() -> ConcurrentlyBuilder {
    info!("Building do_dig_job");
    let dig = Steps::build()
        .label("dig")
        .step(SetDigTarget)
        .step(dig_tile());

    Concurrently::build()
        .mode(ConcurrentMode::Race)
        .label("do_dig_job")
        .push(ChecktileExists)
        .push(dig)
}

#[derive(Component, Debug, Clone, ActionBuilder)]
struct SetDigTarget;

fn set_dig_target(
    mut commands: Commands,
    mut action_query: Query<(&Actor, &mut ActionState, &ActionSpan), With<SetDigTarget>>,
    assigned_job_query: Query<&AssignedJob>,
    dig_job_query: Query<&DigJob>,
) {
    for (actor, mut action_state, span) in &mut action_query {
        let _guard = span.span().enter();

        match *action_state {
            ActionState::Requested => {
                info!("Setting dig target");
                *action_state = ActionState::Executing;
            }
            ActionState::Executing => {
                // Add a DigTarget component to the actor
                let assigned_dig_job = assigned_job_query
                    .get(actor.0)
                    .and_then(|assigned_job| dig_job_query.get(assigned_job.0))
                    .expect("Actor should have an assigned job");

                info!(job=?assigned_dig_job, "Setting dig target");
                commands
                    .entity(actor.0)
                    .insert(DigTarget(assigned_dig_job.0));
                *action_state = ActionState::Success;
            }
            ActionState::Cancelled => {
                info!("Setting dig target cancelled");
                *action_state = ActionState::Failure;
            }
            _ => {}
        }
    }
}

#[derive(Component, Clone, Debug, ScorerBuilder)]
pub struct Digger;

#[derive(Resource, Debug)]
struct DigJobs(HashSet<DigJob>);

fn digger_scorer(
    mut actor_query: Query<(&Actor, &mut Score, &ScorerSpan), With<Digger>>,
    dig_jobs_query: Query<&DigJob>,
    _global_transform_query: Query<&GlobalTransform>,
) {
    for (_actor, mut score, span) in &mut actor_query {
        let _guard = span.span().enter();
        // for now, just return a score of 1.0 when there is a job
        if dig_jobs_query.iter().next().is_some() {
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

        // for DigJob { tile } in &dig_jobs_query {
        //     let tile_position = global_transform_query
        //         .get(*tile)
        //         .unwrap()
        //         .translation()
        //         .xy();

        //     let distance = actor_position.distance(tile_position);

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
struct ChecktileExists;

fn check_tile_exists(
    mut action_query: Query<(&Actor, &mut ActionState, &ActionSpan), With<ChecktileExists>>,
    assigned_job_query: Query<&AssignedJob>,
    dig_job_query: Query<&DigJob>,
    tile_query: Query<&TilePos>,
) {
    for (actor, mut action_state, span) in &mut action_query {
        let _guard = span.span().enter();

        match *action_state {
            ActionState::Requested => {
                info!("Checking tile exists");
                *action_state = ActionState::Executing;
            }
            ActionState::Executing => {
                // Check if the target tile still exists
                if let Ok(AssignedJob(job_entity)) = assigned_job_query.get(actor.0) {
                    if let Ok(DigJob(tile)) = dig_job_query.get(*job_entity) {
                        if tile_query.get(*tile).is_err() {
                            info!("tile does not exist");
                            *action_state = ActionState::Success;
                        } else {
                            debug!("tile still exists");
                        }
                    } else {
                        info!("No dig job");
                        *action_state = ActionState::Cancelled;
                    }
                } else {
                    info!("No assigned job");
                    *action_state = ActionState::Cancelled;
                }
            }
            ActionState::Cancelled => {
                info!("Check tile exists cancelled");
                *action_state = ActionState::Failure;
            }
            _ => {}
        }
    }
}
