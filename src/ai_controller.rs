use bevy::{
    math::{Vec3Swizzles, Vec4Swizzles},
    prelude::*,
};
use bevy_ecs_tilemap::prelude::{SquarePos, TilemapGridSize};
use pathfinding::prelude::astar;

use crate::{
    movement::{MovementSet, Walker},
    pathfinding::find_path,
    terrain::{Terrain, TerrainData},
};

#[derive(Component)]
pub(crate) struct AiControlled;

#[derive(Component, Reflect)]
pub(crate) struct MoveTo {
    pub(crate) entity: Option<Entity>,
    pub(crate) position: Vec2,
}

pub(crate) struct Path {
    pub(crate) path: Vec<UVec2>,
}

fn move_to_target(
    mut target_query: Query<(&MoveTo, &mut Walker, &Transform), With<AiControlled>>,
    terrain_query: Query<(&GlobalTransform, &TerrainData, &TilemapGridSize), With<Terrain>>,
) {
    let Ok((terrain_global_transform, terrain_data, tilemap_grid_size)) = terrain_query.get_single() else { return; };
    for (target, mut walker, walker_transform) in &mut target_query {
        let target_tile_pos_vec4 = terrain_global_transform.compute_matrix().inverse()
            * Vec4::new(target.position.x, target.position.y, 0., 1.);
        let target_tile_pos = (target_tile_pos_vec4 / tilemap_grid_size.x).xy().as_uvec2();

        let walker_pos_in_terrain_transform = terrain_global_transform.compute_matrix().inverse()
            * Vec4::new(
                walker_transform.translation.x,
                walker_transform.translation.y,
                0.,
                1.,
            );

        let walker_square_pos =
            SquarePos::from_world_pos(&walker_pos_in_terrain_transform.xy(), tilemap_grid_size);

        let walker_tile_pos = UVec2::new(walker_square_pos.x as u32, walker_square_pos.y as u32);

        let path = find_path(terrain_data, walker_tile_pos, target_tile_pos);

        if let Some(next_tile) = path.get(1) {
            let next_tile_world_pos = terrain_global_transform.compute_matrix()
                * Vec4::new(
                    next_tile.x as f32 * tilemap_grid_size.x,
                    next_tile.y as f32 * tilemap_grid_size.y,
                    0.,
                    1.,
                );
            let walker_world_pos = terrain_global_transform.compute_matrix()
                * Vec4::new(
                    walker_tile_pos.x as f32 * tilemap_grid_size.x,
                    walker_tile_pos.y as f32 * tilemap_grid_size.y,
                    0.,
                    1.,
                );
            let distance = next_tile_world_pos.xy()
                - Vec2::new(walker_transform.translation.x, walker_world_pos.y);
            walker.move_direction = Some(distance.normalize());
        } else if let Some(last_tile) = path.last() {
            let next_tile_world_pos = terrain_global_transform.compute_matrix()
                * Vec4::new(
                    last_tile.x as f32 * tilemap_grid_size.x,
                    last_tile.y as f32 * tilemap_grid_size.y,
                    0.,
                    1.,
                );
            let walker_world_pos = terrain_global_transform.compute_matrix()
                * Vec4::new(
                    walker_tile_pos.x as f32 * tilemap_grid_size.x,
                    walker_tile_pos.y as f32 * tilemap_grid_size.y,
                    0.,
                    1.,
                );
            let distance = next_tile_world_pos.xy()
                - Vec2::new(walker_transform.translation.x, walker_world_pos.y);
            walker.move_direction = Some(distance.normalize());
        } else {
            walker.move_direction = None;
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

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub(crate) struct AiControllerSet;

pub(crate) struct AiControllerPlugin;

impl Plugin for AiControllerPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<MoveTo>().add_systems(
            (update_target, move_to_target, move_to_removed)
                .chain()
                .in_set(AiControllerSet)
                .before(MovementSet),
        );
    }
}
