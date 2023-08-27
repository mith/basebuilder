use std::marker::PhantomData;

use bevy::{
    ecs::system::SystemParam,
    math::Vec3Swizzles,
    prelude::{Commands, Component, Entity, GlobalTransform, Query, Vec2, With},
    reflect::Reflect,
};
use big_brain::{
    prelude::ScorerBuilder,
    scorers::Score,
    thinker::{Actor, ScorerSpan},
};

use crate::pathfinding::Pathfinding;

#[derive(Component, Clone, Reflect, Debug)]
pub struct ActionArea(pub Vec<Vec2>);

pub trait HasActionPosition {
    type PositionQuery<'w, 's>: SystemParam;

    fn action_pos(&self, query: &Self::PositionQuery<'_, '_>) -> Option<Vec2>;
}

pub trait HasActionArea: HasActionPosition {
    fn action_area(&self, query: &Self::PositionQuery<'_, '_>) -> Option<ActionArea>;
}

#[derive(Clone, Debug)]
pub struct ActionAreaReachableBuilder<T>(PhantomData<T>);

#[derive(Component, Clone, Debug)]
pub struct ActionAreaReachable<T>(PhantomData<T>);

impl<T: Component + HasActionArea> ActionAreaReachable<T> {
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

pub fn action_area_reachable<T: HasActionArea + bevy::prelude::Component>(
    mut actor_query: Query<(&Actor, &mut Score, &ScorerSpan), With<ActionAreaReachable<T>>>,
    global_transform_query: Query<&GlobalTransform>,
    action_pos_query: T::PositionQuery<'_, '_>,
    action_query: Query<(Entity, &T)>,
    pathfinding: Pathfinding,
) {
    for (actor, mut score, span) in &mut actor_query {
        let _guard = span.span().enter();
        let actor_pos = global_transform_query
            .get(actor.0)
            .expect("Actor should have a global transform")
            .translation()
            .xy();

        let actions_iter = action_query.iter();
        let any_reachable_action_area = actions_iter
            .flat_map(|(_action_entity, action)| {
                action
                    .action_area(&action_pos_query)
                    .map(|area| area.0)
                    .unwrap_or_else(Vec::new)
            })
            .any(|tile| pathfinding.find_path(actor_pos, tile).is_some());

        if any_reachable_action_area {
            score.set(1.0);
        } else {
            score.set(0.0);
        }
    }
}
