use bevy::{
    math::{Vec3Swizzles, Vec4Swizzles},
    prelude::*,
};
use bevy_ecs_tilemap::{prelude::TilemapGridSize, tiles::TilePos};

use crate::{
    ai_controller::MoveTo,
    designation_layer::Designated,
    hovered_tile::HoveredTile,
    job::{AssignedTo, HasJob, Job, Worker},
    terrain::{Terrain, TileDamageEvent, TileDestroyedEvent},
};

pub struct DigPlugin;

impl Plugin for DigPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Digging>()
            .register_type::<JobTimer>()
            .add_systems((designate_dig, dig, dig_timer, finish_digging));
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

#[derive(Component, Reflect)]
pub struct Digging(Entity);

#[derive(Component, Reflect)]
pub struct JobTimer(pub Timer);

fn dig(
    mut commands: Commands,
    assigned_dig_job_query: Query<(Entity, &TilePos, &AssignedTo), (With<Job>, With<DigTarget>)>,
    worker_query: Query<&GlobalTransform, (With<Worker>, Without<Digging>)>,
    terrain_tilemap_query: Query<(&Transform, &TilemapGridSize), With<Terrain>>,
) {
    // Make a walk walk to the dig target and start digging
    for (dig_job_entity, dig_target_tile_pos, assigned_to) in &assigned_dig_job_query {
        if let Ok(dwarf_transform) = worker_query.get(assigned_to.entity) {
            for (terrain_transform, tilemap_grid_size) in &terrain_tilemap_query {
                let tile_global_position = (terrain_transform.compute_matrix()
                    * Vec4::new(
                        dig_target_tile_pos.x as f32 * tilemap_grid_size.x,
                        dig_target_tile_pos.y as f32 * tilemap_grid_size.y,
                        0.,
                        1.,
                    ))
                .xy();

                if dwarf_transform
                    .translation()
                    .xy()
                    .distance(tile_global_position)
                    < 20.
                {
                    commands.entity(assigned_to.entity).remove::<MoveTo>();
                    commands.entity(assigned_to.entity).insert((
                        Digging(dig_job_entity),
                        JobTimer(Timer::from_seconds(1., TimerMode::Repeating)),
                    ));
                } else {
                    commands.entity(assigned_to.entity).insert(MoveTo {
                        entity: Some(dig_job_entity),
                        position: tile_global_position,
                    });
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
            commands
                .entity(unassigned_worker_entity)
                .remove::<Digging>()
                .remove::<JobTimer>()
                .remove::<HasJob>();
        }
    }
    for tile_destroyed_event in tile_destroyed_event_reader.iter() {
        for (digging, worker_entity) in &digging_worker_query {
            if digging.0 == tile_destroyed_event.entity {
                commands
                    .entity(worker_entity)
                    .remove::<Digging>()
                    .remove::<JobTimer>()
                    .remove::<HasJob>();
            }
        }
    }
}
