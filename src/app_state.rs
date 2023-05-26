use bevy::prelude::*;

pub(crate) struct AppStatePlugin;

impl Plugin for AppStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<AppState>();
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, States)]
pub(crate) enum AppState {
    #[default]
    Loading,
    Game,
}
