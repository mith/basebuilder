use bevy::{
    math::{Vec3Swizzles, Vec4Swizzles},
    prelude::*,
};
use bevy_ecs_tilemap::{
    prelude::{TilemapGridSize, TilemapSize, TilemapType},
    tiles::{TilePos, TileStorage},
};

use crate::{cursor_position::CursorPosition, terrain::TerrainConfig};

#[derive(Resource)]
pub(crate) struct HoveredTile {
    pub entity: Entity,
    pub tile_center: Vec2,
}

pub(crate) fn hovered_tile(
    terrain_config: Res<TerrainConfig>,
    cursor_pos: Res<CursorPosition>,
    mut hovered_tile: ResMut<HoveredTile>,
    chunks_query: Query<(
        &Transform,
        &TileStorage,
        &TilemapSize,
        &TilemapGridSize,
        &TilemapType,
    )>,
) {
    let cursor_pos = cursor_pos.0;
    for (chunk_transform, tile_storage, chunk_size, grid_size, map_type) in &chunks_query {
        let cursor_in_chunk_pos: Vec2 = {
            // Extend the cursor_pos vec3 by 1.0
            let cursor_pos = Vec4::from((cursor_pos, 1.));
            let cursor_in_chunk_pos = chunk_transform.compute_matrix().inverse() * cursor_pos;
            cursor_in_chunk_pos.xy()
        };

        if let Some(tile_pos) =
            TilePos::from_world_pos(&cursor_in_chunk_pos, chunk_size, grid_size, map_type)
        {
            if let Some(tile_entity) = tile_storage.get(&tile_pos) {
                let tile_center = tile_pos.center_in_world(
                    &TilemapGridSize {
                        x: terrain_config.cell_size,
                        y: terrain_config.cell_size,
                    },
                    map_type,
                );
                *hovered_tile = HoveredTile {
                    entity: tile_entity,
                    tile_center: chunk_transform.translation.xy() + tile_center,
                };
            }
        }
    }
}

pub(crate) struct HoveredTilePlugin;

impl Plugin for HoveredTilePlugin {
    fn build(&self, app: &mut App) {
        app.add_system(hovered_tile);
    }
}
