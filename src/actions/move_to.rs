use bevy::{
    math::Vec3Swizzles,
    prelude::{
        App, Component, Entity, GlobalTransform, IntoSystemConfigs, Plugin, PreUpdate, Query, Vec2,
    },
    reflect::Reflect,
};

use big_brain::{
    prelude::ActionState,
    thinker::{ActionSpan, Actor},
    BigBrainSet,
};
use tracing::{error, info};

use crate::{
    movement::Walker,
    pathfinding::{Path, Pathfinding},
    terrain::TerrainParams,
};

pub struct MoveToPlugin;

impl Plugin for MoveToPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<MoveToPosition>().add_systems(
            PreUpdate,
            (move_to_position, follow_entity).in_set(BigBrainSet::Actions),
        );
    }
}

#[derive(Component, Debug, Reflect)]
pub struct MoveToPosition {
    pub destination: Vec2,
}

#[derive(Component, Debug, Default, Reflect)]
pub struct Moving;

fn move_to_position(
    mut move_to_query: Query<(
        &Actor,
        &mut Walker,
        &mut ActionState,
        &MoveToPosition,
        &ActionSpan,
    )>,
    global_transform_query: Query<&GlobalTransform>,
    pathfinding: Pathfinding,
    terrain: TerrainParams,
) {
    for (actor, mut walker, mut action_state, move_to, span) in &mut move_to_query {
        let _guard = span.span().enter();

        match *action_state {
            ActionState::Requested => {
                info!(traveller=?actor.0, "Requested to move to {:?}", move_to.destination);
                *action_state = ActionState::Executing;
            }
            ActionState::Executing => {
                let actor_position = global_transform_query
                    .get(actor.0)
                    .expect("MoveTo performer entity transform not found")
                    .translation()
                    .xy();

                if (actor_position - move_to.destination).length() < 1. {
                    info!("At destination");
                    *action_state = ActionState::Success;
                    continue;
                }

                let path = pathfinding.find_path(actor_position, move_to.destination);
                if let Some(path) = path {
                    info!("Path found to destination");
                    follow_path(path, &mut walker, actor_position, &terrain);
                } else {
                    error!("No path found to destination");
                    *action_state = ActionState::Failure;
                }
            }
            ActionState::Cancelled => {
                info!("Cancelled");
                *action_state = ActionState::Failure;
            }
            _ => {}
        }
    }
}

#[derive(Component, Debug, Reflect)]
pub struct FollowEntity {
    pub entity: Entity,
    pub distance: f32,
}

fn follow_entity(
    mut follow_entity_query: Query<(
        &Actor,
        &mut Walker,
        &mut ActionState,
        &FollowEntity,
        &ActionSpan,
    )>,
    global_transform_query: Query<&GlobalTransform>,
    pathfinding: Pathfinding,
    terrain: TerrainParams,
) {
    for (actor, mut walker, mut action_state, follow_entity, span) in &mut follow_entity_query {
        let _guard = span.span().enter();

        match *action_state {
            ActionState::Requested => {
                info!(traveller=?actor.0, "Requested to follow {:?}", follow_entity.entity);
                *action_state = ActionState::Executing;
            }
            ActionState::Executing => {
                let actor_position = global_transform_query
                    .get(actor.0)
                    .expect("MoveTo performer entity transform not found")
                    .translation()
                    .xy();

                let destination_position = global_transform_query
                    .get(follow_entity.entity)
                    .expect("MoveTo destination entity transform not found")
                    .translation()
                    .xy();

                if (destination_position - actor_position).length() < follow_entity.distance {
                    info!("At destination");
                    *action_state = ActionState::Success;
                } else {
                    let path = pathfinding.find_path(actor_position, destination_position);
                    if let Some(path) = path {
                        info!("Path found to destination");
                        follow_path(path, &mut walker, actor_position, &terrain);
                    } else {
                        error!("No path found to destination");
                        *action_state = ActionState::Failure;
                    }
                }
            }
            ActionState::Cancelled => {
                info!("Cancelled");
                *action_state = ActionState::Failure;
            }
            _ => {}
        }
    }
}

pub fn follow_path(
    mut path: Path,
    walker: &mut Walker,
    walker_position: Vec2,
    terrain: &TerrainParams,
) {
    let walker_tile_pos = terrain.global_to_tile_pos(walker_position).unwrap();
    // if within the center of the first tile in the path, remove it
    if let Some(&first_tile) = path.0.first() {
        let is_climbing = walker_tile_pos.y != first_tile.y && walker_tile_pos.x == first_tile.x;
        let first_tile_world_pos = terrain.tile_to_global_pos(first_tile.into());
        const TILE_SIZE: f32 = 16.;
        if is_climbing {
            let distance = Vec2::new(
                first_tile_world_pos.x - walker_position.x,
                first_tile_world_pos.y - walker_position.y,
            );
            if distance.length() < TILE_SIZE {
                path.0.remove(0);
            }
        } else {
            let distance = Vec2::new(first_tile_world_pos.x - walker_position.x, 0.);
            if distance.length() < TILE_SIZE {
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
                next_tile_world_pos.x - walker_position.x,
                next_tile_world_pos.y - walker_position.y,
            );
            walker.move_direction = Some(distance.normalize());
        } else {
            let distance = Vec2::new(next_tile_world_pos.x - walker_position.x, 0.);
            walker.move_direction = Some(distance.normalize());
        }
    } else {
        walker.move_direction = None;
    }
}
