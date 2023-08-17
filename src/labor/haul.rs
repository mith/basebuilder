use bevy::prelude::*;

use crate::{
    actions::pickup::Pickup,
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
    for (haul_job_entity, haul_job, AssignedWorker(hauler_entity)) in &mut haul_job_query.iter() {}
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
    haul_job_query: Query<(Entity, &Haul, &AwaitingPickup)>,
) {
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
    for (haul_job_entity, haul_job, AssignedWorker(hauler_entity)) in &haul_job_query {}
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
    haul_job_query: Query<(Entity, &AwaitingPickup, &Haul, Option<&Parent>)>,
    mut haul_complete_event_writer: EventWriter<HaulCompletedEvent>,
) {
}
