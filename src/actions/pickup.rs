use bevy::{math::Vec3Swizzles, prelude::*};
use big_brain::{
    prelude::{ActionBuilder, ActionState},
    thinker::{ActionSpan, Actor},
    BigBrainSet,
};

use crate::labor::job::all_workers_eligible;

pub struct PickupPlugin;

impl Plugin for PickupPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Pickup>()
            .add_systems(Update, all_workers_eligible::<Pickup>)
            .add_systems(PreUpdate, pickup.in_set(BigBrainSet::Actions));
    }
}

#[derive(Component, Debug, Clone, Reflect, ActionBuilder)]
pub struct Pickup {
    pub entity: Entity,
}

fn pickup(
    mut commands: Commands,
    mut pickup_query: Query<(&Actor, &Pickup, &mut ActionState, &ActionSpan)>,
    global_transform_query: Query<&GlobalTransform>,
) {
    for (actor, pickup, mut action_state, span) in &mut pickup_query {
        let _guard = span.span().enter();

        match *action_state {
            ActionState::Requested => {
                info!("Starting pickup");
                *action_state = ActionState::Executing;
            }
            ActionState::Executing => {
                let actor_position = global_transform_query
                    .get(actor.0)
                    .unwrap()
                    .translation()
                    .xy();

                let pickup_position = global_transform_query
                    .get(pickup.entity)
                    .unwrap()
                    .translation()
                    .xy();

                if actor_position.distance(pickup_position) < 16. {
                    // Move item to worker inventory
                    commands.entity(actor.0).add_child(pickup.entity);
                    info!("Picked up");
                    *action_state = ActionState::Success;
                } else {
                    info!("Too far away to pickup");
                    *action_state = ActionState::Failure;
                }
            }
            ActionState::Cancelled => {
                info!("Pickup cancelled");
                *action_state = ActionState::Failure;
            }
            _ => {}
        }
    }
}
