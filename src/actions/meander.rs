use bevy::prelude::{App, Component, IntoSystemConfigs, Plugin, PreUpdate, Query, With};
use big_brain::{
    prelude::{ActionBuilder, ActionState},
    thinker::{ActionSpan, Actor},
    BigBrainSet,
};
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
    _walker_query: Query<&mut Walker>,
) {
    for (_actor, mut action_state, span) in &mut action_query {
        let _guard = span.span().enter();

        match *action_state {
            ActionState::Requested => {
                info!("Starting to meander");
                *action_state = ActionState::Executing;
            }
            ActionState::Executing => {}
            ActionState::Cancelled => {
                info!("Meander cancelled");
                *action_state = ActionState::Failure;
            }
            _ => {}
        }
    }
}
