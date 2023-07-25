use bevy::{
    math::Vec3Swizzles,
    prelude::{
        App, Commands, Component, Entity, GlobalTransform, Parent, Plugin, Query, Update, Vec2,
        Without,
    },
    reflect::Reflect,
};
use tracing::{error, info};

use crate::{actions::action::SuspendedAction, ai_controller::Path, pathfinding::Pathfinding};

use super::action::CompletedAction;

pub struct MoveToPlugin;

impl Plugin for MoveToPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<MoveTo>().add_systems(Update, move_to);
    }
}

#[derive(Component, Debug, Reflect)]
pub struct MoveTo {
    pub to: Vec2,
}

#[derive(Component, Debug, Default, Reflect)]
pub struct Moving;

fn move_to(
    mut commands: Commands,
    move_to_query: Query<(Entity, &MoveTo, &Parent, Option<&Path>)>,
    global_transform_query: Query<&GlobalTransform>,
    _pathfinding: Pathfinding,
) {
    for (move_to_action_entity, move_to, parent, opt_path) in &move_to_query {
        let performer_position = global_transform_query
            .get(parent.get())
            .expect("MoveTo performer entity transform not found")
            .translation()
            .xy();

        if (move_to.to - performer_position).length() < 10. {
            // MoveTo complete
            commands
                .entity(move_to_action_entity)
                .insert(CompletedAction);
        } else if opt_path.is_some() {
            continue;
        } else {
        }
    }
}

pub trait TravelAction {
    type AtTarget: Component + Default;
    type TravelingToTarget: Component + Default;

    fn target_entity(&self) -> Entity;
}

pub fn travel_to_entity<T>(
    mut commands: Commands,
    action_query: Query<
        (Entity, &T, &Parent),
        (Without<T::AtTarget>, Without<T::TravelingToTarget>),
    >,
    global_transform_query: Query<&GlobalTransform>,
    pathfinding: Pathfinding,
) where
    T: Component + TravelAction,
{
    for (action_entity, action, parent) in action_query.iter() {
        let target_entity = action.target_entity();
        {
            let performer_entity = parent.get();
            let pathfinding = &pathfinding;
            let start_position = global_transform_query
                .get(performer_entity)
                .unwrap()
                .translation()
                .xy();

            let destination_position = global_transform_query
                .get(target_entity)
                .unwrap()
                .translation()
                .xy();

            if (destination_position - start_position).length() < 10. {
                info!(traveller=?performer_entity, "At destination");
                commands
                    .entity(performer_entity)
                    .insert(<T::AtTarget>::default());
            } else {
                let path = pathfinding.find_path(start_position, destination_position);
                if let Some(path) = path {
                    info!(traveller=?performer_entity, "Path found to destination");
                    commands.entity(performer_entity).insert(Path(path));
                } else {
                    error!(traveller=?performer_entity, "No path found to destination");
                }
            }
        };

        commands
            .entity(action_entity)
            .insert(T::TravelingToTarget::default());
        info!(traveller=?parent.get(), "Traveling to target");
    }
}

pub fn travel_to_nearby_tiles<T>(
    mut commands: Commands,
    action_query: Query<
        (Entity, &T, &Parent),
        (Without<T::AtTarget>, Without<T::TravelingToTarget>),
    >,
    global_transform_query: Query<&GlobalTransform>,
    pathfinding: Pathfinding,
) where
    T: Component + TravelAction,
{
    for (action_entity, action, parent) in action_query.iter() {
        let target_entity = action.target_entity();

        for x in -1..=1 {
            for y in -1..=1 {
                let destination_position = global_transform_query
                    .get(target_entity)
                    .unwrap()
                    .translation()
                    .xy()
                    + Vec2::new(x as f32, y as f32) * 16.;

                let performer_entity = parent.get();
                let start_position = global_transform_query
                    .get(performer_entity)
                    .unwrap()
                    .translation()
                    .xy();

                if (destination_position - start_position).length() < 10. {
                    info!(traveller=?performer_entity, "At destination");
                    commands
                        .entity(performer_entity)
                        .insert(<T::AtTarget>::default());
                } else {
                    let path = pathfinding.find_path(start_position, destination_position);
                    if let Some(path) = path {
                        info!(traveller=?performer_entity, "Path found to destination");
                        commands.entity(performer_entity).insert(Path(path));
                    } else {
                        error!(
                            traveller=?performer_entity, start = ?start_position, destination = ?destination_position,
                            "No path found to destination"
                        );
                    }
                }
            }
        }

        commands
            .entity(action_entity)
            .insert(T::TravelingToTarget::default());
        info!(traveller=?parent.get(), "Traveling to target");
    }
}
