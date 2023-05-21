use bevy::prelude::*;

pub struct JobPlugin;

impl Plugin for JobPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(assign_job);
    }
}

#[derive(Component)]
pub struct Job;

#[derive(Component)]
pub struct Worker;

#[derive(Component)]
pub struct HasJob;

#[derive(Component)]
pub struct AssignedTo {
    pub entity: Entity,
}

fn assign_job(
    mut commands: Commands,
    unassigned_job_query: Query<Entity, (With<Job>, Without<AssignedTo>)>,
    worker_query: Query<Entity, (With<Worker>, Without<HasJob>)>,
) {
    let mut available_workers = worker_query.iter().collect::<Vec<_>>();
    // Look for unnassigned jobs and assign them to the first unnoccupied worker
    for job_entity in &unassigned_job_query {
        if let Some(available_dwarf) = available_workers.pop() {
            commands.entity(available_dwarf).insert(HasJob);
            commands.entity(job_entity).insert(AssignedTo {
                entity: available_dwarf,
            });
        }
    }
}
