use bevy::prelude::*;
use bevy_ecs_tilemap::tiles::TilePos;

use crate::{
    actions::action_area::ActionArea,
    designation_layer::Designated,
    hovered_tile::HoveredTile,
    labor::job::{all_workers_eligible, Job},
    terrain::TerrainParams,
};

use super::job::{AssignedWorker, CompletedJob};

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
    }
}

#[derive(Component, Debug, Clone, Reflect)]
pub struct DigJob(pub Entity);

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
                    ActionArea(vec![
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
            info!(job=?job_entity, tile=?tile_entity, tile_pos=?tile_pos,  "Designated dig job");
        }
    }
}

#[derive(Component)]
struct AwaitingDig(pub Entity);

fn schedule_dig_action(
    _commands: Commands,
    dig_job_query: Query<
        (Entity, &DigJob, &AssignedWorker, &ActionArea),
        (Without<AwaitingDig>, Without<CompletedJob>),
    >,
) {
    for (_job_entity, DigJob(_tile_entity), AssignedWorker(_worker_entity), _action_area) in
        &mut dig_job_query.iter()
    {}
}

#[derive(Event)]
pub struct DigJobCompleteEvent {
    pub job: Entity,
    pub parent_job: Option<Entity>,
    pub worker: Entity,
    pub tile: Entity,
}

fn finish_digjob(
    _commands: Commands,
    _dig_job_query: Query<(
        Entity,
        &DigJob,
        &AwaitingDig,
        &AssignedWorker,
        Option<&Parent>,
    )>,
    _digging_complete_event_writer: EventWriter<DigJobCompleteEvent>,
) {
}
