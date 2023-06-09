use bevy::prelude::*;

use crate::labor::job::{all_workers_eligible, job_assigned, AssignedJob, AtJobSite, Complete};

pub struct DeliverPlugin;

impl Plugin for DeliverPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<DeliveryCompletedEvent>().add_systems((
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

pub struct DeliveryCompletedEvent {
    pub job: Entity,
    pub parent_job: Option<Entity>,
    pub worker: Entity,
    pub item: Entity,
}

fn complete_delivery(
    mut commands: Commands,
    worker_query: Query<(Entity, &AssignedJob), (With<Delivering>, With<AtJobSite>)>,
    delivery_query: Query<(&Delivery, Option<&Parent>)>,
    mut delivery_complete_event_writer: EventWriter<DeliveryCompletedEvent>,
) {
    for (worker_entity, assigned_job) in &mut worker_query.iter() {
        let (delivery, parent_job) = delivery_query.get(assigned_job.0).unwrap();
        commands.entity(delivery.to).add_child(delivery.load);

        commands.entity(assigned_job.0).insert(Complete);

        delivery_complete_event_writer.send(DeliveryCompletedEvent {
            job: assigned_job.0,
            parent_job: parent_job.map(|e| e.get()),
            worker: worker_entity,
            item: delivery.load,
        });
    }
}
