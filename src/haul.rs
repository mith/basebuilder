use bevy::prelude::*;

use crate::{
    deliver::{Delivering, Delivery},
    job::{all_workers_eligible, AssignedJob, AssignedTo, AtJobSite, Job, JobSite},
    pickup::{Pickup, PickupCompleteEvent},
};

pub struct HaulPlugin;

impl Plugin for HaulPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems((
            all_workers_eligible::<Haul>,
            haul_job_assigned,
            pickup_complete,
            complete_haul,
        ));
    }
}

#[derive(Component)]
pub struct Haul {
    pub load: Entity,
    pub amount: u32,
    pub to: Entity,
    pickup_job: Option<Entity>,
    delivery_job: Option<Entity>,
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
            pickup_job: None,
            delivery_job: None,
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
    mut pickup_complete_event_reader: EventReader<PickupCompleteEvent>,
    mut haul_job_query: Query<&mut Haul>,
) {
    for PickupCompleteEvent { job, worker, item } in pickup_complete_event_reader.iter() {
        for mut haul in &mut haul_job_query {
            if haul.pickup_job == Some(*job) {
                haul.pickup_job = None;
                commands.entity(*worker).with_children(|parent| {
                    haul.delivery_job = Some(
                        parent
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
                            .id(),
                    );
                });
                if haul.delivery_job.is_some() {
                    commands.entity(*worker).insert(Delivering);
                }
            }
        }
    }
}

fn complete_haul(
    mut commands: Commands,
    worker_query: Query<(Entity, &AssignedJob), (With<Hauler>, With<Delivering>, With<AtJobSite>)>,
    delivery_query: Query<&Haul>,
) {
    for (worker_entity, assigned_job) in &mut worker_query.iter() {
        let delivery = delivery_query.get(assigned_job.0).unwrap();
        commands.entity(worker_entity).remove::<Delivering>();
        commands.entity(assigned_job.0).despawn_recursive();
    }
}
