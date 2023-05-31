use bevy::{math::Vec3Swizzles, prelude::*};
use bevy_ecs_tilemap::{
    helpers::square_grid::neighbors::SquareDirection, prelude::TilemapSize, tiles::TilePos,
};

use crate::{
    ai_controller::{MoveTo, Path},
    climbable::ClimbableMap,
    designation_layer::Designated,
    hovered_tile::HoveredTile,
    job::{Accessible, AssignedTo, HasJob, Job, Worker},
    pathfinding::{can_move_to, can_stand_or_climb, find_path},
    terrain::{
        Terrain, TerrainData, TerrainParams, TerrainSet, TileDamageEvent, TileDestroyedEvent,
    },
};

pub struct DigPlugin;

impl Plugin for DigPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Digging>()
            .register_type::<JobTimer>()
            .register_type::<DigToolState>()
            .add_state::<DigToolState>()
            .add_systems((
                designate_dig.run_if(state_exists_and_equals(DigToolState::Designating)),
                check_accessibility.before(TerrainSet),
                dig,
                dig_timer.before(TerrainSet),
                finish_digging,
            ));
    }
}

#[derive(Component)]
pub struct DigTarget;

#[derive(States, Default, Reflect, Clone, Eq, PartialEq, Hash, Debug)]
pub enum DigToolState {
    #[default]
    Inactive,
    Designating,
}

fn designate_dig(
    mut commands: Commands,
    mouse_button_input: Res<Input<MouseButton>>,
    tile_query: Query<Entity, With<HoveredTile>>,
) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        for tile_entity in &tile_query {
            commands
                .entity(tile_entity)
                .insert((Job, DigTarget, Designated));
        }
    }
}

fn check_accessibility(
    mut commands: Commands,
    terrain_tilemap_query: Query<(&TerrainData, &TilemapSize, &ClimbableMap), With<Terrain>>,
    job_query: Query<(Entity, &TilePos), (With<DigTarget>, With<Job>)>,
) {
    for (terrain_data, tilemap_size, _ClimbableMap) in &terrain_tilemap_query {
        for (job_entity, job_tile_pos) in &job_query {
            // check if any of the tiles to the side and top of the job tile are accessible
            let tile_pos_up = job_tile_pos
                .square_offset(&SquareDirection::North, tilemap_size)
                .map(|pos| TilePos::new(pos.x, pos.y));
            let tile_pos_left = job_tile_pos.square_offset(&SquareDirection::West, tilemap_size);
            let tile_pos_right = job_tile_pos.square_offset(&SquareDirection::East, tilemap_size);

            let all_tile_positions: Vec<TilePos> = [tile_pos_up, tile_pos_left, tile_pos_right]
                .iter()
                .flatten()
                .copied()
                .collect();

            let accessible = all_tile_positions.iter().any(|pos| {
                terrain_data
                    .get_tile(pos.into())
                    .map_or(false, |tile| tile == 0)
            });
            if accessible {
                commands.entity(job_entity).insert(Accessible);
            } else {
                commands.entity(job_entity).remove::<Accessible>();
            }
        }
    }
}

#[derive(Component, Reflect)]
pub struct Digging(Entity);

#[derive(Component, Reflect)]
pub struct JobTimer(pub Timer);

fn dig(
    mut commands: Commands,
    assigned_dig_job_query: Query<(Entity, &TilePos, &AssignedTo), (With<Job>, With<DigTarget>)>,
    worker_query: Query<(&GlobalTransform, Option<&Path>), (With<Worker>, Without<Digging>)>,
    terrain: TerrainParams,
    climbable_map: Query<&ClimbableMap, With<Terrain>>,
) {
    // Make a walk walk to the dig target and start digging
    for (dig_job_entity, dig_target_tile_pos, assigned_to) in &assigned_dig_job_query {
        if let Ok((worker_transform, path)) = worker_query.get(assigned_to.entity) {
            let climbable_map = climbable_map.single();
            let terrain_data = terrain.terrain_data_query.single();
            let tile_global_position = terrain.tile_to_global_pos(*dig_target_tile_pos);

            if worker_transform
                .translation()
                .xy()
                .distance(tile_global_position)
                < 26.
            {
                commands
                    .entity(assigned_to.entity)
                    .remove::<MoveTo>()
                    .remove::<Path>();
                commands.entity(assigned_to.entity).insert((
                    Digging(dig_job_entity),
                    JobTimer(Timer::from_seconds(1., TimerMode::Repeating)),
                ));
            } else if path.is_none() {
                let walker_tile_pos = terrain
                    .global_to_tile_pos(worker_transform.translation().xy())
                    .unwrap();

                let map_size: TilemapSize = terrain_data.map_size().into();

                // check if there are any tiles near the dig target that are accessible
                // and if so, make the worker move to them
                for direction in [
                    SquareDirection::North,
                    SquareDirection::NorthEast,
                    SquareDirection::East,
                    SquareDirection::SouthEast,
                    SquareDirection::South,
                    SquareDirection::SouthWest,
                    SquareDirection::West,
                    SquareDirection::NorthWest,
                ]
                .iter()
                {
                    let Some(target_tile_pos) = dig_target_tile_pos.square_offset(direction, &map_size) else {
                        continue;
                    };

                    let can_stand_or_climb_in_target =
                        can_stand_or_climb(terrain_data, Some(climbable_map), target_tile_pos);

                    let path_to_target_tile = find_path(
                        terrain_data,
                        Some(climbable_map),
                        walker_tile_pos.into(),
                        target_tile_pos.into(),
                    );

                    if can_stand_or_climb_in_target && !path_to_target_tile.is_empty() {
                        let tile_global_position = terrain.tile_to_global_pos(target_tile_pos);
                        commands.entity(assigned_to.entity).insert((
                            MoveTo {
                                entity: Some(dig_job_entity),
                                position: tile_global_position,
                            },
                            Path(path_to_target_tile),
                        ));
                        break;
                    }
                }
            }
        }
    }
}

fn dig_timer(
    time: Res<Time>,
    mut digging_worker_query: Query<(&Digging, &mut JobTimer), With<Worker>>,
    mut tile_damage_event_writer: EventWriter<TileDamageEvent>,
) {
    for (digging, mut dig_timer) in &mut digging_worker_query {
        if dig_timer.0.tick(time.delta()).just_finished() {
            tile_damage_event_writer.send(TileDamageEvent {
                tile: digging.0,
                damage: 20,
            });
        }
    }
}

fn finish_digging(
    mut commands: Commands,
    mut tile_destroyed_event_reader: EventReader<TileDestroyedEvent>,
    digging_worker_query: Query<(&Digging, Entity), With<Worker>>,
    mut unassigned_workers: RemovedComponents<HasJob>,
) {
    for unassigned_worker_entity in unassigned_workers.iter() {
        if digging_worker_query.get(unassigned_worker_entity).is_ok() {
            remove_digging_job(&mut commands, unassigned_worker_entity);
        }
    }
    for tile_destroyed_event in tile_destroyed_event_reader.iter() {
        for (digging, worker_entity) in &digging_worker_query {
            if digging.0 == tile_destroyed_event.entity {
                remove_digging_job(&mut commands, worker_entity);
            }
        }
    }
}

fn remove_digging_job(commands: &mut Commands, unassigned_worker_entity: Entity) {
    commands
        .entity(unassigned_worker_entity)
        .remove::<Digging>()
        .remove::<JobTimer>()
        .remove::<HasJob>()
        .remove::<MoveTo>()
        .remove::<Path>();
}
