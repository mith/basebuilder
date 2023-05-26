use bevy::prelude::*;

use pathfinding::directed::astar::astar;

use crate::{climbable::ClimbableMap, terrain::TerrainData};

pub(crate) fn find_path(
    terrain_data: &TerrainData,
    climbable_map: Option<&ClimbableMap>,
    start_tile_pos: UVec2,
    target_tile_pos: UVec2,
) -> Vec<UVec2> {
    astar(
        &start_tile_pos,
        |p| {
            let mut successors = Vec::new();
            for x in -1..=1i32 {
                for y in -1..=1i32 {
                    if x == 0 && y == 0 {
                        continue;
                    }
                    let new_pos = (p.as_ivec2() + IVec2::new(x, y)).as_uvec2();

                    // check if the tile has a value of 0, meaning it is accessible
                    // check if the tile has ground underneath it

                    let new_tile_is_air = terrain_data
                        .0
                        .get([new_pos.x as usize, new_pos.y as usize])
                        .map_or(false, |tile| *tile == 0);

                    if x == 1 && y == 0 && new_tile_is_air {
                        // check if current tile is climbable
                        if let Some(climbable_map) = climbable_map {
                            let current_is_climbable = climbable_map.is_climbable((*p).into());
                            if current_is_climbable {
                                successors.push((new_pos, 1));
                                continue;
                            }
                        }
                    }

                    let ground_underneath_new_tile = terrain_data
                        .0
                        .get([new_pos.x as usize, new_pos.y as usize - 1])
                        .map_or(false, |tile| *tile != 0);
                    if new_tile_is_air && ground_underneath_new_tile {
                        successors.push((new_pos, 1));
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
    .0
}
