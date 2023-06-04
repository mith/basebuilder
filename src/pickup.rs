use bevy::prelude::*;

use crate::job::{all_workers_eligible, job_assigned, AssignedJob, AtJobSite, Worker};

pub struct PickupPlugin;

impl Plugin for PickupPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<PickupCompleteEvent>().add_systems((
            job_assigned::<Pickup, PickingUp>,
            all_workers_eligible::<Pickup>,
            pickup,
        ));
    }
}

#[derive(Component, Default)]
struct PickingUp;

#[derive(Component)]
pub struct Pickup {
    pub amount: u32,
    pub from: Entity,
}

pub struct PickupCompleteEvent {
    pub job: Entity,
    pub worker: Entity,
    pub item: Entity,
}

fn pickup(
    mut commands: Commands,
    worker_query: Query<(Entity, &AssignedJob), (With<Worker>, With<PickingUp>, With<AtJobSite>)>,
    pickup_job_query: Query<&Pickup>,
    mut delivery_query: Query<(Entity, &mut Transform)>,
    mut pickup_complete_event_writer: EventWriter<PickupCompleteEvent>,
) {
    for (worker_entity, assigned_job) in &mut worker_query.iter() {
        let pickup_job = pickup_job_query.get(assigned_job.0).unwrap();
        let (load_entity, mut pickup_transform) = delivery_query.get_mut(pickup_job.from).unwrap();

        // Move item to worker inventory

        commands
            .entity(worker_entity)
            .remove::<PickingUp>()
            .add_child(load_entity);

        *pickup_transform = Transform::from_translation(Vec3::new(0.0, 0.0, 1.0));

        commands.entity(assigned_job.0).despawn_recursive();
        commands
            .entity(worker_entity)
            .remove::<AssignedJob>()
            .remove::<PickingUp>();

        pickup_complete_event_writer.send(PickupCompleteEvent {
            job: assigned_job.0,
            worker: worker_entity,
            item: load_entity,
        });
    }
}
