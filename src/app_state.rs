use bevy::prelude::*;

pub struct AppStatePlugin;

impl Plugin for AppStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<AppState>();
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, States)]
pub enum AppState {
    #[default]
    Loading,
    Game,
}
