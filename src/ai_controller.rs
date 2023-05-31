use bevy::{math::Vec3Swizzles, prelude::*};

use crate::{
    gravity::GravitySet,
    movement::{Climbing, Falling, MovementSet, Walker},
    terrain::{TerrainParams, TerrainSet, TerrainState, TileDestroyedEvent},
};

pub(crate) struct AiControllerPlugin;

impl Plugin for AiControllerPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<MoveTo>()
            .register_type::<Path>()
            .add_systems(
                (
                    invalidate_paths,
                    apply_system_buffers,
                    update_target,
                    move_to_target,
                    move_to_removed,
                )
                    .chain()
                    .in_set(AiControllerSet)
                    .distributive_run_if(in_state(TerrainState::Spawned))
                    .before(GravitySet)
                    .before(MovementSet)
                    .after(TerrainSet),
            );
    }
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub(crate) struct AiControllerSet;

#[derive(Component)]
pub(crate) struct AiControlled;

#[derive(Component, Reflect)]
pub(crate) struct MoveTo {
    pub(crate) entity: Option<Entity>,
    pub(crate) position: Vec2,
}

#[derive(Component, Reflect)]
pub(crate) struct Path(pub(crate) Vec<UVec2>);

fn move_to_target(
    mut target_query: Query<
        (&mut Path, &mut Walker, &GlobalTransform),
        (With<MoveTo>, With<AiControlled>, Without<Falling>),
    >,
    terrain: TerrainParams,
) {
    for (mut path, mut walker, walker_global_transform) in &mut target_query {
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
        } // else: path is empty, destination reached
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

fn update_target(mut target_query: Query<&mut MoveTo>, entity_query: Query<&Transform>) {
    for mut target in &mut target_query {
        if let Some(entity) = target.entity {
            if let Ok(entity_transform) = entity_query.get(entity) {
                target.position = entity_transform.translation.xy();
            }
        }
    }
}

fn move_to_removed(mut removed: RemovedComponents<MoveTo>, mut target_query: Query<&mut Walker>) {
    for entity in &mut removed {
        if let Ok(mut target) = target_query.get_mut(entity) {
            target.move_direction = None;
        }
    }
}
