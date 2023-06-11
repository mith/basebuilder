use bevy::prelude::*;

use crate::labor::job::{all_workers_eligible, AssignedJob, AtJobSite, Complete, Worker};

use super::job::{register_job, JobAssignmentSet};

pub struct PickupPlugin;

impl Plugin for PickupPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<PickupCompletedEvent>()
            .register_type::<Pickup>()
            .add_systems((all_workers_eligible::<Pickup>, pickup).before(JobAssignmentSet));

        register_job::<Pickup, PickingUp>(app);
    }
}

#[derive(Component, Default, Debug)]
pub struct PickingUp;

#[derive(Component, Debug, Clone, Reflect)]
pub struct Pickup {
    pub amount: u32,
    pub from: Entity,
}

pub struct PickupCompletedEvent {
    pub job: Entity,
    pub parent_job: Option<Entity>,
    pub worker: Entity,
    pub item: Entity,
}

fn pickup(
    mut commands: Commands,
    worker_query: Query<(Entity, &AssignedJob), (With<Worker>, With<PickingUp>, With<AtJobSite>)>,
    pickup_job_query: Query<(&Pickup, Option<&Parent>)>,
    mut transform_query: Query<&mut Transform>,
    mut pickup_complete_event_writer: EventWriter<PickupCompletedEvent>,
) {
    for (worker_entity, AssignedJob(pickup_job_entity)) in &mut worker_query.iter() {
        let (pickup_job, parent_job) = pickup_job_query.get(*pickup_job_entity).unwrap();
        let mut pickup_transform = transform_query.get_mut(pickup_job.from).unwrap();
        let load_entity = pickup_job.from;

        // Move item to worker inventory

        commands
            .entity(worker_entity)
            .remove::<PickingUp>()
            .add_child(load_entity);

        *pickup_transform = Transform::from_translation(Vec3::new(0.0, 0.0, 1.0));

        commands.entity(*pickup_job_entity).insert(Complete);

        info!(worker=?worker_entity, pickup_job=?pickup_job, "Pickup complete");

        pickup_complete_event_writer.send(PickupCompletedEvent {
            job: *pickup_job_entity,
            parent_job: parent_job.map(|p| p.get()),
            worker: worker_entity,
            item: load_entity,
        });
    }
}
