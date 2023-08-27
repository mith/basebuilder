use std::marker::PhantomData;

use bevy::{
    ecs::system::{
        lifetimeless::{Read, SQuery},
        StaticSystemParam, SystemParam, SystemParamItem,
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

use crate::pathfinding::Pathfinding;

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

#[derive(SystemParam)]
pub struct ActionAreaParam<'w, 's, T>
where
    T: GlobalActionArea + Component + 'static,
{
    action_query: Query<'w, 's, (Entity, Read<T>)>,
    action_pos_query: StaticSystemParam<'w, 's, <T as HasActionPosition>::PositionParam>,
    pathfinding: Pathfinding<'w, 's>,
}

impl<'w, 's, T> ActionAreaParam<'_, '_, T>
where
    T: GlobalActionArea + Component,
{
    pub fn is_action_area_reachable(&self, actor_pos: Vec2) -> bool {
        let any_reachable_action_area = self
            .action_query
            .iter()
            .flat_map(|(_action_entity, action)| {
                self.global_action_area(action)
                    .map(|area| area.0)
                    .unwrap_or_else(Vec::new)
            })
            .any(|tile| self.pathfinding.find_path(actor_pos, tile).is_some());
        any_reachable_action_area
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
pub fn action_area_reachable<T: GlobalActionArea + bevy::prelude::Component>(
    mut actor_query: Query<(&Actor, &mut Score, &ScorerSpan), With<ActionAreaReachable<T>>>,
    global_transform_query: Query<&GlobalTransform>,
    action_area_param: ActionAreaParam<T>,
) {
    for (actor, mut score, span) in &mut actor_query {
        let _guard = span.span().enter();
        let actor_pos = global_transform_query
            .get(actor.0)
            .expect("Actor should have a global transform")
            .translation()
            .xy();

        let any_reachable_action_area = action_area_param.is_action_area_reachable(actor_pos);

        if any_reachable_action_area {
            score.set(1.0);
        } else {
            score.set(0.0);
        }
    }
}
