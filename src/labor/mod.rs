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
        app.add_plugins((
            JobPlugin,
            StuckPlugin,
            DigPlugin,
            BuildStructurePlugin,
            ChopTreePlugin,
            HaulPlugin,
        ));
    }
}
