use bevy::{
    math::Vec3Swizzles,
    prelude::{
        apply_system_buffers, error, App, Commands, Component, Entity, GlobalTransform,
        IntoSystemConfig, IntoSystemConfigs, Plugin, Query, Without,
    },
};
use tracing::info;

use crate::{
    ai_controller::Path,
    labor::job::{AssignedJob, AtJobSite, JobSite},
    pathfinding::Pathfinding,
};

use super::{
    job::{Complete, JobAssignmentSet},
    stuck::StuckTimer,
};

pub struct CommutePlugin;

impl Plugin for CommutePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            (apply_system_buffers, commute, apply_system_buffers)
                .chain()
                .after(JobAssignmentSet),
        );
    }
}

#[derive(Component)]
pub struct Commuting;

fn commute(
    mut commands: Commands,
    workers_query: Query<
        (Entity, &AssignedJob, &GlobalTransform, Option<&Path>),
        Without<AtJobSite>,
    >,
    job_query: Query<&JobSite, Without<Complete>>,
    pathfinder: Pathfinding,
) {
    for (worker_entity, assigned_job, worker_transform, opt_path) in &workers_query {
        // if the worker already has a path, add commute component and continue
        if opt_path.is_some() {
            commands.entity(worker_entity).insert(Commuting);
            continue;
        }

        let Ok(job_site) = job_query.get(assigned_job.0) else {
            error!("Worker has job without job site");
            continue;
        };

        // check if worker is already at job site
        if job_site.0.iter().any(|job_site_world_pos| {
            worker_transform
                .translation()
                .xy()
                .distance(*job_site_world_pos)
                < 10.
        }) {
            commands
                .entity(worker_entity)
                .insert(AtJobSite)
                .remove::<Commuting>();
            info!("Worker arrived at job site");
            continue;
        }

        // find job site tile with the shortest path from worker position
        let paths = job_site.0.iter().filter_map(|job_site_world_pos| {
            pathfinder.find_path(worker_transform.translation().xy(), *job_site_world_pos)
        });
        let shortest_path = paths.min_by(|path_a, path_b| path_a.len().cmp(&path_b.len()));

        if let Some(path) = shortest_path {
            commands
                .entity(worker_entity)
                .remove::<StuckTimer>()
                .insert(Commuting)
                .insert(Path(path));
            info!("Worker started commuting");
        } else {
            // no path found, start stucktimer
            info!("Worker stuck, starting stuck timer");
            commands.entity(worker_entity).insert(StuckTimer::default());
        }
    }
}
