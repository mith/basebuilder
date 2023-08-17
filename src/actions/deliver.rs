use bevy::{
    prelude::{
        App, BuildChildren, Commands, Component, Entity, GlobalTransform, IntoSystemConfigs,
        Plugin, PreUpdate, Query,
    },
    reflect::Reflect,
};
use big_brain::{
    prelude::{ActionBuilder, ActionState},
    thinker::{ActionSpan, Actor},
    BigBrainSet,
};
use tracing::{error, info};

pub struct DeliverPlugin;

impl Plugin for DeliverPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Deliver>()
            .add_systems(PreUpdate, deliver.in_set(BigBrainSet::Actions));
    }
}

#[derive(Component, Debug, Clone, Reflect, ActionBuilder)]
pub struct Deliver {
    pub load: Entity,
    pub to: Entity,
}

fn deliver(
    mut commands: Commands,
    mut deliver_query: Query<(&Actor, &mut Deliver, &mut ActionState, &ActionSpan)>,
    global_transform_query: Query<&GlobalTransform>,
) {
    for (actor, deliver, mut action_state, span) in &mut deliver_query {
        let _guard = span.span().enter();

        match *action_state {
            ActionState::Requested => {
                info!("Starting delivery");
                *action_state = ActionState::Executing;
            }
            ActionState::Executing => {
                info!("Delivering");
                let actor_position = global_transform_query
                    .get(actor.0)
                    .unwrap()
                    .translation()
                    .truncate();
                let destination_position = global_transform_query
                    .get(deliver.to)
                    .unwrap()
                    .translation()
                    .truncate();

                if actor_position.distance(destination_position) < 16. {
                    info!("Delivered");
                    commands.entity(deliver.to).add_child(deliver.load);
                    *action_state = ActionState::Success;
                } else {
                    error!("Too far away to deliver");
                    *action_state = ActionState::Failure;
                }
            }
            ActionState::Cancelled => {
                info!("Cancelled delivery");
                *action_state = ActionState::Failure;
            }
            _ => {}
        }
    }
}
