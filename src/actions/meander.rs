use bevy::prelude::{App, Component, IntoSystemConfigs, Plugin, PreUpdate, Query, Vec2, With};
use big_brain::{
    prelude::{ActionBuilder, ActionState},
    thinker::{ActionSpan, Actor},
    BigBrainSet,
};
use rand::{distributions::WeightedIndex, prelude::Distribution};
use tracing::info;

use crate::movement::Walker;

pub struct MeanderPlugin;

impl Plugin for MeanderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, meander.in_set(BigBrainSet::Actions));
    }
}

#[derive(Component, Debug, Clone, ActionBuilder)]
pub struct Meander;

fn meander(
    mut action_query: Query<(&Actor, &mut ActionState, &ActionSpan), With<Meander>>,
    mut walker_query: Query<&mut Walker>,
) {
    for (actor, mut action_state, span) in &mut action_query {
        let _guard = span.span().enter();

        match *action_state {
            ActionState::Requested => {
                info!("Starting to meander");
                *action_state = ActionState::Executing;
            }
            ActionState::Executing => {
                let Some(mut walker) = walker_query.get_mut(actor.0).ok() else {
                    *action_state = ActionState::Failure;
                    continue;
                };
                if let Some(current_direction) = walker.move_direction {
                    let dist = WeightedIndex::new(&[70, 1]).unwrap();
                    let mut rng = rand::thread_rng();
                    let new_direction = match dist.sample(&mut rng) {
                        0 => current_direction,
                        1 => Vec2::new(-current_direction.x, 0.),
                        _ => unreachable!(),
                    };
                    walker.move_direction = Some(new_direction);
                } else {
                    walker.move_direction = Some(Vec2::new(0.5, 0.));
                }
            }
            ActionState::Cancelled => {
                info!("Meander cancelled");
                *action_state = ActionState::Failure;
            }
            _ => {}
        }
    }
}
