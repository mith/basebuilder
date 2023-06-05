use bevy::prelude::*;

use crate::{
    deliver::{Delivering, Delivery, DeliveryCompletedEvent},
    job::{all_workers_eligible, unassign_job, AssignedJob, AssignedTo, AtJobSite, Job, JobSite},
    pickup::{Pickup, PickupCompletedEvent},
};

pub struct HaulPlugin;

impl Plugin for HaulPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<HaulCompletedEvent>().add_systems((
            all_workers_eligible::<Haul>,
            haul_job_assigned,
            pickup_complete,
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
struct Hauler;

fn haul_job_assigned(
    mut commands: Commands,
    assigned_job_query: Query<(Entity, &Haul, &AssignedTo, &JobSite), Added<AssignedTo>>,
) {
    for (job_entity, haul, assigned_to, job_site) in &mut assigned_job_query.iter() {
        let mut opt_pickup_job: Option<Entity> = None;
        commands.entity(job_entity).with_children(|parent| {
            let pickup_job = parent
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
            opt_pickup_job = Some(pickup_job);
        });

        if let Some(pickup_job) = opt_pickup_job {
            commands
                .entity(assigned_to.0)
                .insert((Hauler, AssignedJob(pickup_job)));
        }
    }
}

fn pickup_complete(
    mut commands: Commands,
    mut pickup_complete_event_reader: EventReader<PickupCompletedEvent>,
    mut haul_job_query: Query<&mut Haul>,
) {
    for PickupCompletedEvent {
        job: _,
        parent_job,
        worker,
        item,
    } in pickup_complete_event_reader.iter()
    {
        if let Some((haul_entity, haul)) =
            parent_job.and_then(|e| haul_job_query.get_mut(e).ok().map(|h| (e, h)))
        {
            let pickup_job = commands
                .spawn((
                    Job,
                    AssignedTo(*worker),
                    Delivery {
                        amount: 0,
                        load: *item,
                        to: haul.to,
                    },
                    haul.delivery_site.clone(),
                ))
                .id();
            commands.entity(haul_entity).add_child(pickup_job);
            commands
                .entity(*worker)
                .insert(Delivering)
                .insert(AssignedJob(pickup_job))
                .insert(haul.delivery_site.clone());
        }
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
        if let Some((haul_entity, haul_parent)) =
            delivery_parent.and_then(|e| haul_job_query.get_mut(e).ok().map(|p| (e, p)))
        {
            commands
                .entity(*worker)
                .remove::<Delivering>()
                .remove::<AssignedJob>();
            unassign_job(&mut commands, *worker);
            commands.entity(haul_entity).despawn_recursive();
            haul_complete_event_writer.send(HaulCompletedEvent {
                job: haul_entity,
                parent_job: haul_parent.map(|p| p.get()),
                worker: *worker,
                item: *item,
            });
        }
    }
}
