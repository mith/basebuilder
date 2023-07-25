use bevy::{
    prelude::{App, Commands, Component, Entity, Plugin, Query, Res, Update, With},
    time::{Time, Timer, TimerMode},
    utils::HashMap,
};
use bevy_rapier2d::prelude::KinematicCharacterControllerOutput;

use crate::{
    ai_controller::Path,
    labor::job::{AssignedJob, AssignedWorker, BlacklistedWorkers, Job, Worker},
};

pub struct StuckPlugin;

impl Plugin for StuckPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (stuck, stuck_timer));
    }
}

#[derive(Component)]
pub struct StuckTimer(Timer);

impl Default for StuckTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(1., TimerMode::Once))
    }
}

fn stuck(
    mut commands: Commands,
    worker_query: Query<
        (
            Entity,
            Option<&StuckTimer>,
            Option<&Path>,
            &KinematicCharacterControllerOutput,
        ),
        (With<Worker>, With<AssignedJob>),
    >,
) {
    for (worker_entity, opt_stuck_timer, opt_path, controller_output) in &worker_query {
        if opt_path.is_none() && opt_stuck_timer.is_some() {
            commands.entity(worker_entity).remove::<StuckTimer>();
            continue;
        }

        let standing_still = controller_output.effective_translation.x == 0.
            && controller_output.effective_translation.y == 0.;
        if !standing_still && opt_stuck_timer.is_some() {
            commands.entity(worker_entity).remove::<StuckTimer>();
        } else if standing_still && opt_stuck_timer.is_none() {
            commands
                .entity(worker_entity)
                .insert(StuckTimer(Timer::from_seconds(1., TimerMode::Once)));
        }
    }
}

fn stuck_timer(
    mut commands: Commands,
    time: Res<Time>,
    mut stuck_timer_query: Query<(Entity, &mut StuckTimer, &AssignedJob), With<Worker>>,
    mut blacklisted_workers_query: Query<&mut BlacklistedWorkers, With<Job>>,
) {
    for (worker_entity, mut stuck_timer, assigned_job) in &mut stuck_timer_query {
        if stuck_timer.0.tick(time.delta()).just_finished() {
            commands.entity(worker_entity).remove::<StuckTimer>();
            commands.entity(worker_entity).remove::<AssignedJob>();

            commands.entity(assigned_job.0).remove::<AssignedWorker>();
            if let Ok(mut blacklisted_workers) = blacklisted_workers_query.get_mut(assigned_job.0) {
                blacklisted_workers
                    .0
                    .insert(worker_entity, Timer::from_seconds(5., TimerMode::Once));
            } else {
                let map =
                    HashMap::from([(worker_entity, Timer::from_seconds(5., TimerMode::Once))]);
                commands
                    .entity(assigned_job.0)
                    .insert(BlacklistedWorkers(map));
            }
        }
    }
}
