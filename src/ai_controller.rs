use bevy::{
    math::{Vec3Swizzles, Vec4Swizzles},
    prelude::*,
};

use crate::{
    climbable::ClimbableMap,
    movement::{MovementSet, Walker},
    pathfinding::find_path,
    terrain::{Terrain, TerrainParams},
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
    mut target_query: Query<(&MoveTo, &mut Walker, &GlobalTransform), With<AiControlled>>,
    terrain: TerrainParams,
    climbable_map: Query<&ClimbableMap, With<Terrain>>,
) {
    let Ok(climbable_map) = climbable_map.get_single() else { return; };
    let Ok(terrain_data) = terrain.terrain_data_query.get_single() else { return; };
    for (target, mut walker, walker_global_transform) in &mut target_query {
        let target_tile_pos = terrain.global_to_tile_pos(target.position).unwrap();
        let walker_tile_pos = terrain
            .global_to_tile_pos(walker_global_transform.translation().xy())
            .unwrap();

        let path = find_path(
            terrain_data,
            Some(climbable_map),
            walker_tile_pos.into(),
            target_tile_pos.into(),
        );

        if let Some(next_tile) = path.get(1).copied() {
            let next_tile_world_pos = terrain.tile_to_global_pos(next_tile.into());
            let walker_tile_world_pos = terrain.tile_to_global_pos(walker_tile_pos.into());
            // if next tile position is above or below walker position, first move to the center of the current tile
            let distance_from_center =
                (walker_global_transform.translation().x - walker_tile_world_pos.x).abs();
            if next_tile.y != walker_tile_pos.y
                && next_tile.x == walker_tile_pos.x
                && distance_from_center > 1.
            {
                let distance = Vec2::new(
                    walker_tile_world_pos.x - walker_global_transform.translation().x,
                    0.,
                );
                walker.move_direction = Some(distance.normalize());
            } else {
                let distance = next_tile_world_pos - walker_tile_world_pos;
                walker.move_direction = Some(distance.normalize());
            }
        } else if let Some(last_tile) = path.last().copied() {
            let next_tile_world_pos = terrain.tile_to_global_pos(last_tile.into());
            let walker_tile_world_pos = terrain.tile_to_global_pos(walker_tile_pos.into());
            let distance = next_tile_world_pos
                - Vec2::new(
                    walker_global_transform.translation().x,
                    walker_tile_world_pos.y,
                );
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
