use bevy::prelude::{
    App, Commands, Component, IntoSystemConfigs, Plugin, PreUpdate, Query, With, Without,
};
use big_brain::{
    actions::ConcurrentlyBuilder,
    prelude::{ActionBuilder, ActionState, ConcurrentMode, Concurrently, Steps},
    thinker::{ActionSpan, Actor},
    BigBrainSet,
};
use tracing::{debug, info};

use crate::{
    actions::fell::FellTarget,
    labor::{
        chop_tree::FellingJob,
        job::{AssignedJob, AssignedWorker},
    },
    tree::Tree,
};

use super::{action_area::action_area_reachable, fell::fell_tree};
pub struct DoFellingJobPlugin;

impl Plugin for DoFellingJobPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            action_area_reachable::<FellingJob, Without<AssignedWorker>>
                .in_set(BigBrainSet::Scorers),
        )
        .add_systems(
            PreUpdate,
            (set_fell_target, check_tree_exists).in_set(BigBrainSet::Actions),
        );
    }
}

pub fn do_fell_job() -> ConcurrentlyBuilder {
    info!("Building do_fell_job action");
    let fell = Steps::build()
        .label("fell")
        .step(SetFellTarget)
        .step(fell_tree());

    Concurrently::build()
        .mode(ConcurrentMode::Race)
        .label("do_fell_job")
        .push(CheckTreeExists)
        .push(fell)
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
