use std::marker::PhantomData;

use bevy::{
    math::Vec3Swizzles,
    prelude::{
        Commands, Component, Entity, GlobalTransform, IntoSystemConfigs, Plugin, PreUpdate, Query,
        Vec2, With,
    },
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

pub trait HasActionArea {
    fn action_area(action_pos: Vec2) -> ActionArea;
    fn action_pos(&self, global_transform_query: &Query<&GlobalTransform>) -> Option<Vec2>;
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
        let _num_actions = actions_iter.len();
        let any_reachable_action_area = actions_iter
            .flat_map(|(_action_entity, action)| {
                if let Some(action_global_pos) = action.action_pos(&global_transform_query) {
                    T::action_area(action_global_pos).0
                } else {
                    Vec::new()
                }
            })
            .any(|tile| pathfinding.find_path(actor_pos, tile).is_some());

        if any_reachable_action_area {
            score.set(1.0);
        } else {
            score.set(0.0);
        }
    }
}
