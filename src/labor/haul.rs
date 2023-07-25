use bevy::prelude::*;

use crate::{
    actions::{
        action::{Action, ActionCompletedEvent},
        deliver::Deliver,
        pickup::Pickup,
    },
    labor::job::{all_workers_eligible, Complete, JobSite},
};

use super::job::{AssignedWorker, JobAssignmentSet};

pub struct HaulPlugin;

impl Plugin for HaulPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<HaulCompletedEvent>().add_systems(
            Update,
            (
                all_workers_eligible::<Haul>,
                start_haul_job,
                pickup_complete,
                start_delivery,
                delivery_complete,
            )
                .before(JobAssignmentSet),
        );
    }
}

#[derive(Component)]
pub struct Haul {
    pub load: Entity,
    pub amount: u32,
    pub to: Entity,
    pickup_site: JobSite,
    delivery_site: JobSite,
}

impl Haul {
    pub fn new(
        load: Entity,
        amount: u32,
        to: Entity,
        pickup_site: JobSite,
        delivery_site: JobSite,
    ) -> Self {
        Self {
            load,
            amount,
            to,
            pickup_site,
            delivery_site,
        }
    }
}

#[derive(Component, Default)]
pub struct Hauler;

#[derive(Component)]
struct AwaitingPickup(Entity);

/// Start a pickup job when a haul job is assigned and the worker is not already carrying the load
/// (i.e. the load is not a child of the worker)
fn start_haul_job(
    mut commands: Commands,
    haul_job_query: Query<
        (Entity, &Haul, &AssignedWorker),
        (
            Without<AwaitingPickup>,
            Without<AwaitingDelivery>,
            Without<Complete>,
        ),
    >,
    children_query: Query<&Children>,
) {
    for (haul_job_entity, haul_job, AssignedWorker(hauler_entity)) in &mut haul_job_query.iter() {
        // check if the worker is already carrying the load
        if is_hauler_carrying_load(&children_query, *hauler_entity, haul_job.load) {
            debug!(worker=?hauler_entity, load = ?haul_job.load, "Worker is already carrying the load.");
            commands.entity(haul_job_entity).insert(WorkerCarrying);
            continue;
        }
        let pickup_action = commands
            .spawn((
                Action,
                Pickup {
                    amount: haul_job.amount,
                    from: haul_job.load,
                },
                haul_job.pickup_site.clone(),
            ))
            .id();
        commands.entity(*hauler_entity).add_child(pickup_action);

        commands
            .entity(haul_job_entity)
            .insert(AwaitingPickup(pickup_action));

        info!(
            action=?pickup_action, job=?haul_job_entity, pickup=?haul_job.pickup_site,
            "Spawned pickup action for haul job."
        );
    }
}

/// Check if a hauler is carrying an item
fn is_hauler_carrying_load(
    children_query: &Query<&Children>,
    hauler: Entity,
    load: Entity,
) -> bool {
    children_query.iter_descendants(hauler).any(|e| e == load)
}

#[derive(Component, Default)]
pub struct WorkerCarrying;

fn pickup_complete(
    mut commands: Commands,
    mut pickup_complete_event_reader: EventReader<ActionCompletedEvent<Pickup>>,
    haul_job_query: Query<(Entity, &Haul, &AwaitingPickup)>,
) {
    for ActionCompletedEvent {
        action_entity: completed_action_entity,
        performer_entity,
        action: Pickup {
            amount,
            from: pickup_up_entity,
        },
    } in pickup_complete_event_reader.iter()
    {
        if let Some((haul_job_entity, haul_job)) = haul_job_query.iter().find_map(
            |(haul_job_entity, haul, AwaitingPickup(queued_action_entity))| {
                if completed_action_entity == queued_action_entity {
                    Some((haul_job_entity, haul))
                } else {
                    None
                }
            },
        ) {
            if haul_job.load != *pickup_up_entity {
                panic!("Haul job load does not match pickup action load.")
            }
            if haul_job.amount != *amount {
                panic!("Haul job amount does not match pickup action amount.")
            }
            commands
                .entity(haul_job_entity)
                .insert(WorkerCarrying)
                .remove::<AwaitingPickup>();
            info!(
                "Hauler {:?} picked up {:?} for haul job {:?}",
                performer_entity, pickup_up_entity, haul_job_entity
            );
        }
    }
}

#[derive(Component)]
struct AwaitingDelivery(Entity);

fn start_delivery(
    mut commands: Commands,
    haul_job_query: Query<
        (Entity, &Haul, &AssignedWorker),
        (With<WorkerCarrying>, Without<AwaitingDelivery>),
    >,
) {
    for (haul_job_entity, haul_job, AssignedWorker(hauler_entity)) in &haul_job_query {
        let delivery_action = commands
            .spawn((
                Action,
                Deliver {
                    load: haul_job.load,
                    to: haul_job.to,
                },
                haul_job.delivery_site.clone(),
            ))
            .id();
        commands.entity(*hauler_entity).add_child(delivery_action);

        commands
            .entity(haul_job_entity)
            .insert(AwaitingDelivery(delivery_action));

        info!(
            action=?delivery_action, job=?haul_job_entity, delivery=?haul_job.delivery_site,
            "Spawned delivery action for haul job."
        );
    }
}

#[derive(Event)]
pub struct HaulCompletedEvent {
    pub job: Entity,
    pub parent_job: Option<Entity>,
    pub worker: Entity,
    pub item: Entity,
}

fn delivery_complete(
    mut commands: Commands,
    mut delivery_complete_event_reader: EventReader<ActionCompletedEvent<Deliver>>,
    haul_job_query: Query<(Entity, &AwaitingPickup, &Haul, Option<&Parent>)>,
    mut haul_complete_event_writer: EventWriter<HaulCompletedEvent>,
) {
    for ActionCompletedEvent::<Deliver> {
        action_entity: completed_action_entity,
        performer_entity: worker,
        action: Deliver { load: item, to },
    } in delivery_complete_event_reader.iter()
    {
        if let Some((haul_job_entity, haul_job, haul_job_parent)) = haul_job_query.iter().find_map(
            |(haul_job_entity, AwaitingPickup(queued_action_entity), haul_job, haul_job_parent)| {
                if completed_action_entity == queued_action_entity {
                    Some((haul_job_entity, haul_job, haul_job_parent))
                } else {
                    None
                }
            },
        ) {
            if haul_job.to != *to {
                error!(
                    "Haul job {:?} to {:?} does not match delivery action to {:?}.",
                    haul_job_entity, haul_job.to, to
                );
                continue;
            }
            // Mark job as complete
            commands
                .entity(haul_job_entity)
                .remove::<AwaitingDelivery>()
                .remove::<WorkerCarrying>()
                .insert(Complete);

            info!(worker=?worker, haul_job=?haul_job_entity, "Haul job completed");

            haul_complete_event_writer.send(HaulCompletedEvent {
                job: haul_job_entity,
                parent_job: haul_job_parent.map(|p| p.get()),
                worker: *worker,
                item: *item,
            });
        }
    }
}
