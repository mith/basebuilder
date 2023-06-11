use bevy::prelude::*;

use crate::labor::job::{all_workers_eligible, AssignedJob, AssignedTo, Complete, Job, JobSite};

use super::{
    deliver::{Delivering, Delivery, DeliveryCompletedEvent},
    job::{JobAssignedEvent, JobAssignmentSet, JobManagerParams},
    pickup::{PickingUp, Pickup, PickupCompletedEvent},
};

pub struct HaulPlugin;

impl Plugin for HaulPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<HaulCompletedEvent>().add_systems(
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

/// Start a pickup job when a haul job is assigned and the worker is not already carrying the load
/// (i.e. the load is not a child of the worker)
fn start_haul_job(
    mut commands: Commands,
    hauler_query: Query<
        (Entity, &AssignedJob),
        (With<Hauler>, Without<Carrying>, Without<PickingUp>),
    >,
    haul_job_query: Query<(&Haul, &JobSite)>,
    children_query: Query<&Children>,
) {
    for (worker_entity, AssignedJob(job_entity)) in &hauler_query {
        let Ok((haul, job_site)) = haul_job_query.get(*job_entity) else {
            error!("Haul job {:?} not found", job_entity);
            continue;
        };
        // check if the worker is already carrying the load
        if is_carrying(&children_query, worker_entity, haul.load) {
            info!(worker=?worker_entity, "Worker is already carrying the load");
            commands.entity(worker_entity).insert(Carrying);
            continue;
        }
        let pickup_job = commands
            .spawn((
                Job,
                Pickup {
                    amount: haul.amount,
                    from: haul.load,
                },
                job_site.clone(),
            ))
            .id();
        commands.entity(*job_entity).add_child(pickup_job);

        info!(
            "Spawned pickup job {:?} for haul job {:?} at {:?}",
            pickup_job, job_entity, job_site
        );
    }
}

/// Check if a hauler is carrying an item
fn is_carrying(children_query: &Query<&Children>, hauler: Entity, item: Entity) -> bool {
    children_query.iter_descendants(hauler).any(|e| e == item)
}

#[derive(Component, Default)]
pub struct Carrying;

fn pickup_complete(
    mut commands: Commands,
    mut pickup_complete_event_reader: EventReader<PickupCompletedEvent>,
    haul_job_query: Query<&Haul>,
) {
    for PickupCompletedEvent {
        job: _,
        parent_job,
        worker,
        item: _,
    } in pickup_complete_event_reader.iter()
    {
        if parent_job
            .map(|e| haul_job_query.contains(e))
            .unwrap_or(false)
        {
            info!(
                "Worker {:?} completed pickup and is now carrying the load",
                worker
            );
            commands.entity(*worker).insert(Carrying);
        }
    }
}

fn start_delivery(
    mut commands: Commands,
    haul_job_query: Query<&Haul>,
    hauler_query: Query<
        (Entity, &AssignedJob),
        (With<Hauler>, With<Carrying>, Without<Delivering>),
    >,
) {
    for (hauler_entity, assigned_job) in &hauler_query {
        let Ok(haul_job) = haul_job_query.get(assigned_job.0) else {
            error!("Hauler {:?} has no haul job", hauler_entity);
            continue;
        };
        let haul_job_entity = assigned_job.0;
        let delivery_job = commands
            .spawn((
                Job,
                Delivery {
                    amount: 0,
                    load: haul_job.load,
                    to: haul_job.to,
                },
                haul_job.delivery_site.clone(),
            ))
            .id();
        commands.entity(haul_job_entity).add_child(delivery_job);
        info!(
            "Spawned delivery job {:?} for haul job {:?} at {:?}",
            delivery_job, haul_job_entity, haul_job.delivery_site
        );
    }
}

pub struct HaulCompletedEvent {
    pub job: Entity,
    pub parent_job: Option<Entity>,
    pub worker: Entity,
    pub item: Entity,
}

fn delivery_complete(
    mut commands: Commands,
    mut delivery_complete_event_reader: EventReader<DeliveryCompletedEvent>,
    mut haul_job_query: Query<Option<&Parent>, With<Haul>>,
    mut haul_complete_event_writer: EventWriter<HaulCompletedEvent>,
) {
    for DeliveryCompletedEvent {
        job: _,
        parent_job: delivery_parent,
        worker,
        item,
    } in delivery_complete_event_reader.iter()
    {
        if let Some((haul_job_entity, haul_job_parent)) =
            delivery_parent.and_then(|e| haul_job_query.get_mut(e).ok().map(|p| (e, p)))
        {
            // Mark job as complete
            commands.entity(haul_job_entity).insert(Complete);
            // Remove hauler and carrying from worker
            commands
                .entity(*worker)
                .remove::<Carrying>()
                .remove::<Hauler>();

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
