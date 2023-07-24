use bevy::prelude::{App, Plugin};

pub mod action;
pub mod deliver;
pub mod dig;
pub mod fell;
pub mod move_to;
pub mod pickup;

pub struct ActionsPlugin;

impl Plugin for ActionsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(action::ActionPlugin)
            .add_plugin(dig::DigPlugin)
            .add_plugin(pickup::PickupPlugin)
            .add_plugin(deliver::DeliverPlugin)
            .add_plugin(fell::FellPlugin)
            .add_plugin(move_to::MoveToPlugin);
    }
}
