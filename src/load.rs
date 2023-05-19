use bevy::prelude::*;

use crate::{
    app_state::AppState, material::MaterialsState, terrain_settings::TerrainSettingsState,
};

pub(crate) struct LoadPlugin;

fn start_game(mut state: ResMut<NextState<AppState>>) {
    state.set(AppState::Game);
}

impl Plugin for LoadPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(
            start_game
                .run_if(in_state(AppState::Loading))
                .run_if(in_state(MaterialsState::Loaded))
                .run_if(in_state(TerrainSettingsState::Loaded)), // .run_if(in_state(ItemsState::Loaded)),
        );
    }
}
