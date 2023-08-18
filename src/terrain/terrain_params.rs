use bevy::{
    ecs::system::SystemParam,
    math::Vec3Swizzles,
    prelude::{Entity, GlobalTransform, Query, Vec2, With},
};
use bevy_ecs_tilemap::{
    prelude::{TilemapGridSize, TilemapSize, TilemapType},
    tiles::{TilePos, TileStorage},
};

use super::{Terrain, TerrainData};

#[derive(SystemParam)]
pub struct TerrainParams<'w, 's> {
    pub terrain_query: Query<
        'w,
        's,
        (
            Entity,
            &'static GlobalTransform,
            &'static TilemapGridSize,
            &'static TilemapSize,
            &'static TilemapType,
        ),
        With<Terrain>,
    >,
    pub terrain_data_query: Query<'w, 's, &'static TerrainData, With<Terrain>>,
    pub tile_storage: Query<'w, 's, &'static TileStorage, With<Terrain>>,
}

impl TerrainParams<'_, '_> {
    pub fn global_to_tile_pos(&self, global_pos: Vec2) -> Option<TilePos> {
        let Ok((_, terrain_transform, tilemap_grid_size, tilemap_size, tilemap_type)) =
            self.terrain_query.get_single() else {
                return None;
            };
        let global_pos_to_terrain_pos = terrain_transform
            .compute_matrix()
            .inverse()
            .transform_point3(global_pos.extend(0.))
            .xy();
        TilePos::from_world_pos(
            &global_pos_to_terrain_pos,
            tilemap_size,
            tilemap_grid_size,
            tilemap_type,
        )
    }

    pub fn tile_to_global_pos(&self, tile_pos: TilePos) -> Vec2 {
        let (_, terrain_transform, tilemap_grid_size, _tilemap_size, tilemap_type) =
            self.terrain_query.single();
        terrain_transform
            .compute_matrix()
            .transform_point3(
                tile_pos
                    .center_in_world(tilemap_grid_size, tilemap_type)
                    .extend(0.),
            )
            .xy()
    }

    pub fn get_tile_entity(&self, tile_pos: TilePos) -> Option<Entity> {
        let tile_storage = self.tile_storage.single();
        tile_storage.get(&tile_pos)
    }
}
