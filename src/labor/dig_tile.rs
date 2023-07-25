use bevy::prelude::*;
use bevy_ecs_tilemap::tiles::TilePos;

use crate::{
    actions::{
        action::{Action, ActionCompletedEvent},
        dig::Dig,
    },
    designation_layer::Designated,
    hovered_tile::HoveredTile,
    labor::job::{all_workers_eligible, Job, JobSite},
    terrain::TerrainParams,
};

use super::job::{register_job, AssignedWorker, Complete};

pub struct DigPlugin;

impl Plugin for DigPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<DigJobCompleteEvent>()
            .register_type::<DigJob>()
            .register_type::<DigToolState>()
            .add_state::<DigToolState>()
            .add_systems(
                Update,
                (
                    designate_dig.run_if(state_exists_and_equals(DigToolState::Designating)),
                    all_workers_eligible::<DigJob>,
                    schedule_dig_action,
                    finish_digjob,
                ),
            );

        register_job::<DigJob, Digger>(app);
    }
}

#[derive(Component, Debug, Clone, Reflect)]
pub struct DigJob(pub Entity);

#[derive(Component, Default, Debug)]
pub struct Digger;

#[derive(States, Default, Reflect, Clone, Eq, PartialEq, Hash, Debug)]
pub enum DigToolState {
    #[default]
    Inactive,
    Designating,
}

fn designate_dig(
    mut commands: Commands,
    mouse_button_input: Res<Input<MouseButton>>,
    tile_query: Query<(Entity, &TilePos), With<HoveredTile>>,
    terrain: TerrainParams,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        for (tile_entity, tile_pos) in &tile_query {
            let tile_translation = terrain.tile_to_global_pos(*tile_pos);
            let x = tile_translation.x;
            let y = tile_translation.y;
            commands.entity(tile_entity).insert(Designated);
            let job_entity = commands
                .spawn((
                    Job,
                    DigJob(tile_entity),
                    JobSite(vec![
                        // West
                        Vec2::new(x - 16., y),
                        // East
                        Vec2::new(x + 16., y),
                        // South
                        Vec2::new(x, y - 16.),
                        // Northwest
                        Vec2::new(x - 16., y + 16.),
                        // Southwest
                        Vec2::new(x - 16., y - 16.),
                        // Southeast
                        Vec2::new(x + 16., y - 16.),
                        // Northeast
                        Vec2::new(x + 16., y + 16.),
                    ]),
                ))
                .id();
            info!(job = ?job_entity, tile = ?tile_entity, "Designated dig job");
        }
    }
}

#[derive(Component)]
struct AwaitingDig(pub Entity);

fn schedule_dig_action(
    mut commands: Commands,
    dig_job_query: Query<
        (Entity, &DigJob, &AssignedWorker, &JobSite),
        (Without<AwaitingDig>, Without<Complete>),
    >,
) {
    for (job_entity, DigJob(tile_entity), AssignedWorker(worker_entity), job_site) in
        &mut dig_job_query.iter()
    {
        let dig_action = commands
            .spawn((Action, Dig(*tile_entity), job_site.clone()))
            .id();

        commands.entity(job_entity).insert(AwaitingDig(dig_action));

        commands.entity(*worker_entity).add_child(dig_action);

        info!(
            worker = ?worker_entity, job = ?job_entity, action = ?dig_action, tile = ?tile_entity,
            "Scheduled dig action"
        );
    }
}

#[derive(Event)]
pub struct DigJobCompleteEvent {
    pub job: Entity,
    pub parent_job: Option<Entity>,
    pub worker: Entity,
    pub tile: Entity,
}

fn finish_digjob(
    mut commands: Commands,
    dig_job_query: Query<(
        Entity,
        &DigJob,
        &AwaitingDig,
        &AssignedWorker,
        Option<&Parent>,
    )>,
    mut digging_complete_event_reader: EventReader<ActionCompletedEvent<Dig>>,
    mut digging_complete_event_writer: EventWriter<DigJobCompleteEvent>,
) {
    for ActionCompletedEvent::<Dig> {
        action_entity: completed_action_entity,
        performer_entity,
        action: Dig(tile_entity),
    } in digging_complete_event_reader.iter()
    {
        if let Some((
            dig_job_entity,
            DigJob(designated_tile_entity),
            _,
            AssignedWorker(worker_entity),
            parent,
        )) = dig_job_query
            .iter()
            .find(|(_, _, AwaitingDig(awaited_action_entity), _, _)| {
                awaited_action_entity == completed_action_entity
            })
        {
            debug_assert!(
                performer_entity == worker_entity,
                "Dig action was performed by a different entity than the one assigned to the job"
            );

            debug_assert!(
                tile_entity == designated_tile_entity,
                "Dig action was performed on a different tile than the one assigned to the job"
            );

            commands
                .entity(dig_job_entity)
                .remove::<AwaitingDig>()
                .insert(Complete);

            info!(
                worker = ?worker_entity, job = ?dig_job_entity, tile = ?tile_entity,
                "Dig job completed"
            );

            digging_complete_event_writer.send(DigJobCompleteEvent {
                job: dig_job_entity,
                parent_job: parent.map(|p| p.get()),
                worker: *worker_entity,
                tile: *tile_entity,
            });
        }
    }
}
