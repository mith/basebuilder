use bevy::prelude::*;

use build_structure::BuildStructurePlugin;
use chop_tree::ChopTreePlugin;
use dig_tile::DigPlugin;
use haul::HaulPlugin;
use stuck::StuckPlugin;

use job::JobPlugin;

pub mod build_structure;
pub mod chop_tree;
pub mod dig_tile;
pub mod haul;
pub mod job;
pub mod stuck;

pub struct LaborPlugin;

impl Plugin for LaborPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(JobPlugin)
            .add_plugin(StuckPlugin)
            .add_plugin(DigPlugin)
            .add_plugin(BuildStructurePlugin)
            .add_plugin(ChopTreePlugin)
            .add_plugin(HaulPlugin);
    }
}
