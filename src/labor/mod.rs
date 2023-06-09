use bevy::prelude::*;

use build::BuildPlugin;
use chop_tree::FellingPlugin;
use commute::CommutePlugin;
use deliver::DeliverPlugin;
use dig::DigPlugin;
use haul::HaulPlugin;
use pickup::PickupPlugin;
use stuck::StuckPlugin;

use job::JobPlugin;

pub mod build;
pub mod chop_tree;
pub mod commute;
pub mod deliver;
pub mod dig;
pub mod haul;
pub mod job;
pub mod pickup;
pub mod stuck;

pub struct LaborPlugin;

impl Plugin for LaborPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(JobPlugin)
            .add_plugin(CommutePlugin)
            .add_plugin(StuckPlugin)
            .add_plugin(DigPlugin)
            .add_plugin(BuildPlugin)
            .add_plugin(FellingPlugin)
            .add_plugin(PickupPlugin)
            .add_plugin(DeliverPlugin)
            .add_plugin(HaulPlugin);
    }
}
