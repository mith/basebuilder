use bevy::prelude::*;

use crate::labor::job::{
    all_workers_eligible, assign_job, AssignedJob, AssignedTo, Complete, Job, JobSite,
};

use super::{
    deliver::{Delivering, Delivery, DeliveryCompletedEvent},
    pickup::{Pickup, PickupCompletedEvent},
};

pub struct HaulPlugin;

impl Plugin for HaulPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<HaulCompletedEvent>().add_systems((
            all_workers_eligible::<Haul>,
            haul_job_assigned,
            pickup_complete,
            start_delivery,
            delivery_complete,
        ));
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

fn haul_job_assigned(
    mut commands: Commands,
    assigned_job_query: Query<
        (Entity, &Haul, &AssignedTo, &JobSite, Option<&Children>),
        Added<AssignedTo>,
    >,
) {
    for (job_entity, haul, assigned_to, job_site, opt_children) in &mut assigned_job_query.iter() {
        // check if the worker is already carrying the load by checking
        // if the load is a child of the worker
        if opt_children
            .map(|children| children.iter().any(|e| *e == haul.load))
            .unwrap_or(false)
        {
            commands.entity(assigned_to.0).insert(Carrying);
            continue;
        }
        let pickup_job = commands
            .spawn((
                Job,
                AssignedTo(assigned_to.0),
                Pickup {
                    amount: haul.amount,
                    from: haul.load,
                },
                job_site.clone(),
            ))
            .id();

        commands.entity(job_entity).add_child(pickup_job);

        commands
            .entity(assigned_to.0)
            .insert((Hauler, AssignedJob(pickup_job)));
    }
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
        let pickup_job = commands
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
        commands.entity(haul_job_entity).add_child(pickup_job);

        assign_job(&mut commands, hauler_entity, haul_job_entity);
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

            haul_complete_event_writer.send(HaulCompletedEvent {
                job: haul_job_entity,
                parent_job: haul_job_parent.map(|p| p.get()),
                worker: *worker,
                item: *item,
            });
        }
    }
}
