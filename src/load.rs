use bevy::prelude::*;

use crate::{
    main_state::MainState, material::MaterialsState, terrain_settings::TerrainSettingsState,
};

pub struct LoadPlugin;

impl Plugin for LoadPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(
            start_map_generation
                .run_if(in_state(MainState::Loading))
                .run_if(in_state(MaterialsState::Loaded))
                .run_if(in_state(TerrainSettingsState::Loaded)), // .run_if(in_state(ItemsState::Loaded)),
        );
    }
}

fn start_map_generation(mut state: ResMut<NextState<MainState>>) {
    info!("Loading complete");
    state.set(MainState::MapGeneration);
}
