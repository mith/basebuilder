use bevy::{math::Vec3Swizzles, prelude::*, utils::HashSet};
use bevy_ecs_tilemap::tiles::TilePos;

use crate::terrain::{Terrain, TerrainParams};

pub struct ClimbablePlugin;

impl Plugin for ClimbablePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems((create_climbable_map, update_climbable_map));
    }
}

#[derive(Component)]
pub struct Climbable;

#[derive(Component, Default)]
pub struct ClimbableMap(HashSet<TilePos>);

impl ClimbableMap {
    pub fn mark_climbable(&mut self, tile_pos: TilePos) {
        self.0.insert(tile_pos);
    }

    pub fn is_climbable(&self, tile_pos: TilePos) -> bool {
        self.0.contains(&tile_pos)
    }
}

fn create_climbable_map(
    mut commands: Commands,
    mut terrain_query: Query<Entity, (Added<Terrain>, Without<ClimbableMap>)>,
) {
    for entity in &mut terrain_query {
        commands.entity(entity).insert(ClimbableMap::default());
    }
}

fn update_climbable_map(
    mut climbable_map: Query<&mut ClimbableMap, With<Terrain>>,
    terrain: TerrainParams,
    addded_climbabes: Query<&GlobalTransform, Added<Climbable>>,
) {
    let Ok(mut climbable_map) = climbable_map.get_single_mut() else { return; };
    for added_climbable in &addded_climbabes {
        if let Some(climbable_tile_pos) =
            terrain.global_to_tile_pos(added_climbable.translation().xy())
        {
            climbable_map.mark_climbable(climbable_tile_pos.into());
        }
    }
}
