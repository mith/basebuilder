use bevy::prelude::*;

use crate::job::{all_workers_eligible, job_assigned, AssignedJob, AtJobSite, JobAssignedEvent};

pub struct DeliverPlugin;

impl Plugin for DeliverPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems((
            all_workers_eligible::<Delivery>,
            job_assigned::<Delivery, Delivering>,
            complete_delivery,
        ));
    }
}

#[derive(Component)]
pub struct Delivery {
    pub load: Entity,
    pub amount: u32,
    pub to: Entity,
}

#[derive(Component, Default)]
pub struct Delivering;

fn complete_delivery(
    mut commands: Commands,
    worker_query: Query<(Entity, &AssignedJob), (With<Delivering>, With<AtJobSite>)>,
    delivery_query: Query<&Delivery>,
) {
    for (worker_entity, assigned_job) in &mut worker_query.iter() {
        let delivery = delivery_query.get(assigned_job.0).unwrap();
        commands.entity(worker_entity).remove::<Delivering>();
        commands.entity(assigned_job.0).despawn_recursive();
    }
}
