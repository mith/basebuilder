use std::marker::PhantomData;

use bevy::{
    ecs::{
        query::ReadOnlyWorldQuery,
        system::{
            lifetimeless::{Read, SQuery},
            StaticSystemParam, SystemParam, SystemParamItem,
        },
    },
    math::Vec3Swizzles,
    prelude::{Commands, Component, Entity, GlobalTransform, Query, Vec2, With},
    reflect::Reflect,
};
use big_brain::{
    prelude::ScorerBuilder,
    scorers::Score,
    thinker::{Actor, ScorerSpan},
};

use crate::pathfinding::{Path, Pathfinding};

#[derive(Component, Clone, Reflect, Debug)]
pub struct ActionArea(pub Vec<Vec2>);

impl ActionArea {
    pub fn offset(&self, offset: Vec2) -> ActionArea {
        ActionArea(self.0.iter().map(|v| *v + offset).collect())
    }
}

pub trait HasActionPosition {
    type PositionParam: SystemParam;

    fn action_pos(&self, query: &SystemParamItem<Self::PositionParam>) -> Option<Vec2>;
}

pub trait HasActionArea {
    fn action_area() -> ActionArea;
}

pub trait GlobalActionArea: HasActionArea + HasActionPosition {
    fn global_action_area(
        &self,
        query: &SystemParamItem<Self::PositionParam>,
    ) -> Option<ActionArea>;
}

impl<T> GlobalActionArea for T
where
    T: HasActionArea + HasActionPosition,
{
    fn global_action_area(
        &self,
        query: &SystemParamItem<Self::PositionParam>,
    ) -> Option<ActionArea> {
        self.action_pos(query)
            .map(|pos| Self::action_area().offset(pos))
    }
}

#[derive(Clone, Debug)]
pub struct ActionAreaReachableBuilder<T, F = ()>(PhantomData<(T, F)>);

#[derive(Component, Clone, Debug)]
pub struct ActionAreaReachable<T, F = ()>(PhantomData<(T, F)>);

impl<T, F> ActionAreaReachable<T, F>
where
    T: Component + HasActionArea,
    F: ReadOnlyWorldQuery,
{
    pub fn build() -> ActionAreaReachableBuilder<T> {
        ActionAreaReachableBuilder(PhantomData)
    }
}

impl<T: HasActionArea + std::fmt::Debug + Component> ScorerBuilder
    for ActionAreaReachableBuilder<T>
{
    fn build(&self, cmd: &mut Commands, scorer: Entity, _actor: Entity) {
        cmd.entity(scorer)
            .insert(ActionAreaReachable::<T>(PhantomData));
    }
}

#[derive(SystemParam)]
pub struct ActionAreaParam<'w, 's, T, F = ()>
where
    T: GlobalActionArea + Component + 'static,
    F: ReadOnlyWorldQuery + 'static,
{
    action_query: Query<'w, 's, (Entity, Read<T>), F>,
    action_pos_query: StaticSystemParam<'w, 's, <T as HasActionPosition>::PositionParam>,
    pathfinding: Pathfinding<'w, 's>,
}

impl<'w, 's, T, F> ActionAreaParam<'_, '_, T, F>
where
    T: GlobalActionArea + Component,
    F: ReadOnlyWorldQuery + 'static,
{
    pub fn path_to_action_area(&self, actor_pos: Vec2, action: &T) -> Option<Path> {
        self.global_action_area(action).and_then(|area| {
            area.0
                .iter()
                .flat_map(|tile| self.pathfinding.find_path(actor_pos, *tile))
                .min_by_key(|path| path.0.len())
        })
    }

    pub fn closest_action(&self, actor_pos: Vec2) -> Option<(Entity, &T, Path)> {
        self.action_query
            .iter()
            .flat_map(|(entity, action)| {
                self.path_to_action_area(actor_pos, action)
                    .map(|path| (entity, action, path))
            })
            .min_by_key(|(_, _, path)| path.0.len())
    }

    pub fn global_action_area(&self, action: &T) -> Option<ActionArea> {
        action
            .action_pos(&self.action_pos_query)
            .map(|pos| T::action_area().offset(pos))
    }
}

/// Checks if any action of type T is reachable for the actor.
///
/// # Panics
///
/// Panics if the actor does not have a GlobalTransform component.
pub fn action_area_reachable<T, F>(
    mut actor_query: Query<(&Actor, &mut Score, &ScorerSpan), With<ActionAreaReachable<T>>>,
    global_transform_query: Query<&GlobalTransform>,
    action_area_param: ActionAreaParam<T, F>,
) where
    T: GlobalActionArea + Component,
    F: ReadOnlyWorldQuery + 'static,
{
    for (actor, mut score, span) in &mut actor_query {
        let _guard = span.span().enter();
        let actor_pos = global_transform_query
            .get(actor.0)
            .expect("Actor should have a global transform")
            .translation()
            .xy();

        let closest_action_path_length = action_area_param
            .action_query
            .iter()
            .flat_map(|(_, action)| {
                let path = action_area_param.path_to_action_area(actor_pos, action)?;
                Some(path.0.len())
            })
            .min();

        // Convert path length to a score clamped between 0 and 1,
        // where 0 is the longest path and 1 is a path length of zero.

        if let Some(closest_action_path_length) = closest_action_path_length {
            let path_score = 1.0 - (closest_action_path_length as f32 / 100.0).min(0.9);
            debug_assert!(path_score >= 0.1 && path_score <= 1.0);
            score.set(path_score);
        } else {
            score.set(0.0);
        }
    }
}
