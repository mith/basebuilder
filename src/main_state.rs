use bevy::prelude::*;

pub struct MainStatePlugin;

impl Plugin for MainStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<MainState>();
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, States)]
pub enum MainState {
    #[default]
    Loading,
    MapGeneration,
    Game,
}
