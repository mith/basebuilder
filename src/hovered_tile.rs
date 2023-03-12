use bevy::{math::Vec4Swizzles, prelude::*};
use bevy_ecs_tilemap::{
    prelude::{TilemapGridSize, TilemapSize, TilemapType},
    tiles::{TileColor, TilePos, TileStorage},
};

use crate::cursor_position::CursorPosition;

#[derive(Component)]
pub(crate) struct HoveredTile;

fn hovered_tile(
    mut commands: Commands,
    cursor_pos: Res<CursorPosition>,
    hovered_tiles_query: Query<Entity, With<HoveredTile>>,
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

        if let Some(tile_entity) =
            TilePos::from_world_pos(&cursor_in_chunk_pos, chunk_size, grid_size, map_type)
                .as_ref()
                .and_then(|tile_pos| tile_storage.get(tile_pos))
        {
            commands.entity(tile_entity).insert(HoveredTile);
            for hovered_tile in &mut hovered_tiles_query.iter() {
                if hovered_tile != tile_entity {
                    commands.entity(hovered_tile).remove::<HoveredTile>();
                }
            }
        } else {
            for hovered_tile in &mut hovered_tiles_query.iter() {
                commands.entity(hovered_tile).remove::<HoveredTile>();
            }
        }
    }
}

const HIGHLIGHT_COLOR: Color = Color::rgb(1., 1., 0.2);

fn highlight_hovered_tile(mut tile_query: Query<&mut TileColor, Added<HoveredTile>>) {
    for mut tile_color in &mut tile_query {
        tile_color.0 = HIGHLIGHT_COLOR;
    }
}

fn unhighlight_hovered_tile(
    mut hovered_tiles_removed: RemovedComponents<HoveredTile>,
    mut tile_query: Query<&mut TileColor>,
) {
    for tile_entity in hovered_tiles_removed.iter() {
        if let Ok(mut tile_color) = tile_query.get_mut(tile_entity) {
            tile_color.0 = Color::WHITE;
        }
    }
}

pub(crate) struct HoveredTilePlugin;

impl Plugin for HoveredTilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            (
                hovered_tile,
                apply_system_buffers,
                highlight_hovered_tile,
                unhighlight_hovered_tile,
            )
                .chain(),
        );
    }
}
