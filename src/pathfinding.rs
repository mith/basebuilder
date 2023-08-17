use std::matches;

use bevy::{ecs::system::SystemParam, prelude::*};

use bevy_ecs_tilemap::{
    helpers::square_grid::neighbors::SquareDirection, prelude::TilemapSize, tiles::TilePos,
};
use pathfinding::directed::astar::astar;

use crate::{
    climbable::ClimbableMap,
    terrain::{TerrainData, TerrainParams},
};

#[derive(SystemParam)]
pub struct Pathfinding<'w, 's> {
    pub terrain: TerrainParams<'w, 's>,
    climbable_map_query: Query<'w, 's, &'static ClimbableMap>,
}

impl<'w, 's> Pathfinding<'w, 's> {
    pub fn find_path(&self, start_pos: Vec2, target_pos: Vec2) -> Option<Path> {
        let Some(start_tile_pos) = self.terrain.global_to_tile_pos(start_pos) else {
            return None;
        };
        let Some(target_tile_pos) = self.terrain.global_to_tile_pos(target_pos) else {
            return None;
        };
        let terrain_data = self.terrain.terrain_data_query.single();
        let climbable_map = self.climbable_map_query.single();
        let path = find_path(
            terrain_data,
            Some(climbable_map),
            start_tile_pos.into(),
            target_tile_pos.into(),
        );
        path
    }
}

#[derive(Component, Reflect, Clone)]
pub struct Path(pub Vec<UVec2>);

pub fn find_path(
    terrain_data: &TerrainData,
    climbable_map: Option<&ClimbableMap>,
    start_tile_pos: UVec2,
    target_tile_pos: UVec2,
) -> Option<Path> {
    let path = astar(
        &start_tile_pos,
        |p| {
            let mut successors = Vec::new();
            let tile_pos: TilePos = (*p).into();
            let map_size: TilemapSize = terrain_data.map_size().into();
            for direction in [
                SquareDirection::North,
                SquareDirection::NorthEast,
                SquareDirection::East,
                SquareDirection::SouthEast,
                SquareDirection::South,
                SquareDirection::SouthWest,
                SquareDirection::West,
                SquareDirection::NorthWest,
            ]
            .iter()
            {
                let Some(target_tile_pos) = tile_pos.square_offset(direction, &map_size) else {
                    continue;
                };
                if matches!(
                    direction,
                    SquareDirection::SouthWest | SquareDirection::SouthEast
                ) {
                    if can_stand(terrain_data, target_tile_pos)
                        && can_move_to(terrain_data, climbable_map, tile_pos, *direction)
                    {
                        successors.push((target_tile_pos.into(), 1));
                    }
                } else {
                    if can_stand_or_climb(terrain_data, climbable_map, target_tile_pos)
                        && can_move_to(terrain_data, climbable_map, tile_pos, *direction)
                    {
                        successors.push((target_tile_pos.into(), 1));
                    }
                }
            }
            successors
        },
        |p| {
            (p.x as i32 - target_tile_pos.x as i32).abs()
                + (p.y as i32 - target_tile_pos.y as i32).abs()
        },
        |p| *p == target_tile_pos,
    )
    .unwrap_or_default()
    .0;

    if path.is_empty() {
        None
    } else {
        Some(Path(path))
    }
}

pub fn can_stand_or_climb(
    terrain_data: &TerrainData,
    climbable_map: Option<&ClimbableMap>,
    tile_pos: TilePos,
) -> bool {
    let tile_is_empty = terrain_data
        .get_tile(tile_pos.into())
        .map_or(false, |tile| tile == 0);

    if !tile_is_empty {
        return false;
    }

    let can_climb_in_tile = can_climb(climbable_map, tile_pos);

    let can_stand_in_tile = can_stand(terrain_data, tile_pos);

    can_climb_in_tile || can_stand_in_tile
}

pub fn can_climb(climbable_map: Option<&ClimbableMap>, tile_pos: TilePos) -> bool {
    if let Some(climbable_map) = climbable_map {
        let tile_is_climbable = climbable_map.is_climbable(tile_pos.into());
        if tile_is_climbable {
            return true;
        }
    }
    false
}

pub fn can_stand(terrain_data: &TerrainData, tile_pos: TilePos) -> bool {
    let map_size: TilemapSize = terrain_data.map_size().into();
    if terrain_data
        .get_tile(tile_pos.into())
        .map_or(false, |tile| tile != 0)
    {
        // Tile is solid
        return false;
    }
    let Some(south_tile_pos) = tile_pos.square_offset(&SquareDirection::South, &map_size) else {
        // Tileposition is outside of the map
        return false;
    };
    let south_tile_is_solid = terrain_data
        .get_tile(south_tile_pos.into())
        .map_or(false, |tile| tile != 0);
    return south_tile_is_solid;
}

pub fn can_move_to(
    terrain_data: &TerrainData,
    climbable_map: Option<&ClimbableMap>,
    tile_pos: TilePos,
    direction: SquareDirection,
) -> bool {
    let map_size: TilemapSize = terrain_data.map_size().into();
    let Some(new_tile_pos) = tile_pos.square_offset(&direction, &map_size) else {
        // Tileposition is outside of the map
        return false;
    };
    let new_tile_is_empty = terrain_data
        .get_tile(new_tile_pos.into())
        .map_or(false, |tile| tile == 0);

    if !new_tile_is_empty {
        return false;
    }

    if let Some(climbable_map) = climbable_map {
        if direction == SquareDirection::South {
            // if moving to tile above, check if current tile is climbable
            let current_is_climbable = climbable_map.is_climbable(tile_pos.into());
            if current_is_climbable {
                return true;
            }
        } else if direction == SquareDirection::North {
            // if moving to tile below, check if next tile is climbable
            let next_is_climbable = climbable_map.is_climbable(new_tile_pos.into());
            return next_is_climbable;
        }
    }

    match direction {
        SquareDirection::NorthEast | SquareDirection::NorthWest => {
            let Some(north_tile_pos) = tile_pos.square_offset(&SquareDirection::North, &map_size) else {
                // Tileposition is outside of the map
                return false;
            };
            let north_tile_is_empty = terrain_data
                .get_tile(north_tile_pos.into())
                .map_or(false, |tile| tile == 0);
            return north_tile_is_empty;
        }

        SquareDirection::SouthEast => {
            let Some(east_tile_pos) = tile_pos.square_offset(&SquareDirection::East, &map_size) else {
                // Tileposition is outside of the map
                return false;
            };

            let east_tile_is_empty = terrain_data
                .get_tile(east_tile_pos.into())
                .map_or(false, |tile| tile == 0);
            return east_tile_is_empty;
        }
        SquareDirection::SouthWest => {
            let Some(west_tile_pos) = tile_pos.square_offset(&SquareDirection::West, &map_size) else {
                // Tileposition is outside of the map
                return false;
            };

            let west_tile_is_empty = terrain_data
                .get_tile(west_tile_pos.into())
                .map_or(false, |tile| tile == 0);
            return west_tile_is_empty;
        }
        SquareDirection::West | SquareDirection::East => {
            let Some(_target_tile_pos) = tile_pos.square_offset(&direction, &map_size) else {
                // Tileposition is outside of the map
                return false;
            };

            return true;
        }
        _ => {
            return false;
        }
    }
}
