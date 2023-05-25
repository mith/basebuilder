use bevy::{
    math::{Vec3Swizzles, Vec4Swizzles},
    prelude::*,
};
use bevy_ecs_tilemap::{
    helpers::square_grid::neighbors::SquareDirection,
    prelude::{SquarePos, TilemapGridSize, TilemapSize},
    tiles::{TilePos, TileStorage},
};

use crate::{
    ai_controller::MoveTo,
    designation_layer::Designated,
    hovered_tile::HoveredTile,
    job::{Accessible, AssignedTo, HasJob, Job, Worker},
    pathfinding::find_path,
    terrain::{Terrain, TerrainData, TerrainSet, TileDamageEvent, TileDestroyedEvent},
};

pub struct DigPlugin;

impl Plugin for DigPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Digging>()
            .register_type::<JobTimer>()
            .add_systems((
                designate_dig,
                check_accessibility.before(TerrainSet),
                dig,
                dig_timer.before(TerrainSet),
                finish_digging,
            ));
    }
}

#[derive(Component)]
pub struct DigTarget;

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
    terrain_tilemap_query: Query<(&TerrainData, &TilemapSize), With<Terrain>>,
    job_query: Query<(Entity, &TilePos), (With<DigTarget>, With<Job>)>,
) {
    for (terrain_data, tilemap_size) in &terrain_tilemap_query {
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
                    .0
                    .get([pos.x as usize, pos.y as usize])
                    .map_or(false, |tile| *tile == 0)
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
    worker_query: Query<&GlobalTransform, (With<Worker>, Without<Digging>)>,
    terrain_tilemap_query: Query<(&GlobalTransform, &TilemapGridSize, &TerrainData), With<Terrain>>,
) {
    // Make a walk walk to the dig target and start digging
    for (dig_job_entity, dig_target_tile_pos, assigned_to) in &assigned_dig_job_query {
        if let Ok(worker_transform) = worker_query.get(assigned_to.entity) {
            for (terrain_global_transform, tilemap_grid_size, terrain_data) in
                &terrain_tilemap_query
            {
                let tile_global_position = (terrain_global_transform.compute_matrix()
                    * Vec4::new(
                        dig_target_tile_pos.x as f32 * tilemap_grid_size.x,
                        dig_target_tile_pos.y as f32 * tilemap_grid_size.y,
                        0.,
                        1.,
                    ))
                .xy();

                if worker_transform
                    .translation()
                    .xy()
                    .distance(tile_global_position)
                    < 26.
                {
                    commands.entity(assigned_to.entity).remove::<MoveTo>();
                    commands.entity(assigned_to.entity).insert((
                        Digging(dig_job_entity),
                        JobTimer(Timer::from_seconds(1., TimerMode::Repeating)),
                    ));
                } else {
                    let walker_pos_in_terrain_transform =
                        terrain_global_transform.compute_matrix().inverse()
                            * Vec4::new(
                                worker_transform.translation().x,
                                worker_transform.translation().y,
                                0.,
                                1.,
                            );

                    let walker_square_pos = SquarePos::from_world_pos(
                        &walker_pos_in_terrain_transform.xy(),
                        tilemap_grid_size,
                    );

                    let walker_tile_pos =
                        UVec2::new(walker_square_pos.x as u32, walker_square_pos.y as u32);

                    // check if there are any tiles near the dig target that are accessible
                    // and if so, make the worker move to them
                    for x in -1..2i32 {
                        for y in -1..2i32 {
                            if x == 0 && y == 0 {
                                continue;
                            }
                            let p = TilePos::new(
                                (dig_target_tile_pos.x as i32 + x) as u32,
                                (dig_target_tile_pos.y as i32 + y) as u32,
                            );
                            let p_is_air = terrain_data
                                .0
                                .get([p.x as usize, p.y as usize])
                                .map_or(false, |tile| *tile == 0);
                            let ground_underneath_p = terrain_data
                                .0
                                .get([p.x as usize, p.y as usize - 1])
                                .map_or(false, |tile| *tile != 0);
                            let p_is_next_to_target =
                                (p.x as i32 - dig_target_tile_pos.x as i32).abs() <= 1
                                    && (p.y as i32 - dig_target_tile_pos.y as i32).abs() <= 1;
                            let path_to_p =
                                find_path(&terrain_data, walker_tile_pos, UVec2::new(p.x, p.y));
                            if p_is_air
                                && ground_underneath_p
                                && p_is_next_to_target
                                && !path_to_p.is_empty()
                            {
                                let tile_global_position = (terrain_global_transform
                                    .compute_matrix()
                                    * Vec4::new(
                                        p.x as f32 * tilemap_grid_size.x,
                                        p.y as f32 * tilemap_grid_size.y,
                                        0.,
                                        1.,
                                    ))
                                .xy();
                                commands.entity(assigned_to.entity).insert(MoveTo {
                                    entity: Some(dig_job_entity),
                                    position: tile_global_position,
                                });
                                return;
                            }
                        }
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
        .remove::<MoveTo>();
}
