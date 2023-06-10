use bevy::{math::Vec3Swizzles, prelude::*};

use crate::{
    gravity::GravitySet,
    main_state::MainState,
    movement::{Falling, MovementSet, Walker},
    terrain::{TerrainParams, TerrainSet, TileDestroyedEvent},
};

pub struct AiControllerPlugin;

impl Plugin for AiControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ArrivedAtTargetEvent>()
            .register_type::<Path>()
            .add_systems(
                (invalidate_paths, apply_system_buffers, follow_path)
                    .chain()
                    .in_set(AiControllerSet)
                    .distributive_run_if(in_state(MainState::Game))
                    .before(GravitySet)
                    .before(MovementSet)
                    .after(TerrainSet),
            );
    }
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct AiControllerSet;

#[derive(Component)]
pub struct AiControlled;

#[derive(Component, Reflect, Clone, FromReflect)]
pub struct Path(pub Vec<UVec2>);

pub struct ArrivedAtTargetEvent(pub Entity);

fn follow_path(
    mut commands: Commands,
    mut path_query: Query<
        (Entity, &mut Path, &mut Walker, &GlobalTransform),
        (With<AiControlled>, Without<Falling>),
    >,
    terrain: TerrainParams,
    mut arrived_events_writer: EventWriter<ArrivedAtTargetEvent>,
) {
    for (walker_entity, mut path, mut walker, walker_global_transform) in &mut path_query {
        let walker_tile_pos = terrain
            .global_to_tile_pos(walker_global_transform.translation().xy())
            .unwrap();
        // if within the center of the first tile in the path, remove it
        if let Some(&first_tile) = path.0.first() {
            let is_climbing =
                walker_tile_pos.y != first_tile.y && walker_tile_pos.x == first_tile.x;
            let first_tile_world_pos = terrain.tile_to_global_pos(first_tile.into());
            if is_climbing {
                let distance = Vec2::new(
                    first_tile_world_pos.x - walker_global_transform.translation().x,
                    first_tile_world_pos.y - walker_global_transform.translation().y,
                );
                if distance.length() < 1. {
                    path.0.remove(0);
                }
            } else {
                let distance = Vec2::new(
                    first_tile_world_pos.x - walker_global_transform.translation().x,
                    0.,
                );
                if distance.length() < 1. {
                    path.0.remove(0);
                }
            }
        }

        if let Some(&next_tile) = path.0.get(0) {
            let is_climbing = walker_tile_pos.y != next_tile.y && walker_tile_pos.x == next_tile.x;
            // when climbing, move to the center of the tile on both x and y axis
            let next_tile_world_pos = terrain.tile_to_global_pos(next_tile.into());
            if is_climbing {
                let distance = Vec2::new(
                    next_tile_world_pos.x - walker_global_transform.translation().x,
                    next_tile_world_pos.y - walker_global_transform.translation().y,
                );
                walker.move_direction = Some(distance.normalize());
            } else {
                let distance = Vec2::new(
                    next_tile_world_pos.x - walker_global_transform.translation().x,
                    0.,
                );
                walker.move_direction = Some(distance.normalize());
            }
        } else {
            walker.move_direction = None;
            commands.entity(walker_entity).remove::<Path>();
            arrived_events_writer.send(ArrivedAtTargetEvent(walker_entity));
        }
    }
}

fn invalidate_paths(
    mut commands: Commands,
    path_entity_query: Query<Entity, With<Path>>,
    destroyed_tiles: EventReader<TileDestroyedEvent>,
) {
    if !destroyed_tiles.is_empty() {
        for entity in &mut path_entity_query.iter() {
            commands.entity(entity).remove::<Path>();
        }
    }
}
