use bevy::prelude::*;

use crate::labor::job::{
    all_workers_eligible, job_assigned, remove_commute, AssignedJob, AtJobSite, Complete, JobSite,
    Worker,
};

pub struct PickupPlugin;

impl Plugin for PickupPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<PickupCompletedEvent>().add_systems((
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
    for (worker_entity, assigned_job) in &mut worker_query.iter() {
        let (pickup_job, parent_job) = pickup_job_query.get(assigned_job.0).unwrap();
        let mut pickup_transform = transform_query.get_mut(pickup_job.from).unwrap();
        let load_entity = pickup_job.from;

        // Move item to worker inventory

        commands
            .entity(worker_entity)
            .remove::<PickingUp>()
            .add_child(load_entity);

        *pickup_transform = Transform::from_translation(Vec3::new(0.0, 0.0, 1.0));

        commands.entity(assigned_job.0).insert(Complete);

        pickup_complete_event_writer.send(PickupCompletedEvent {
            job: assigned_job.0,
            parent_job: parent_job.map(|p| p.get()),
            worker: worker_entity,
            item: load_entity,
        });
    }
}
