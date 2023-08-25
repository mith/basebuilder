use bevy::prelude::{App, Plugin};

pub mod action_area;
pub mod deliver;
pub mod dig;
pub mod do_dig_job;
pub mod do_fell_job;
pub mod fell;
pub mod meander;
pub mod move_to;
pub mod pickup;
pub mod work;

pub struct ActionsPlugin;

impl Plugin for ActionsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            dig::DigPlugin,
            pickup::PickupPlugin,
            deliver::DeliverPlugin,
            fell::FellPlugin,
            move_to::MoveToPlugin,
            work::WorkPlugin,
            do_fell_job::DoFellingJobPlugin,
            do_dig_job::DoDigJobPlugin,
            meander::MeanderPlugin,
        ));
    }
}
