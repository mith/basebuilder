use bevy::prelude::*;
use bevy_ecs_tilemap::tiles::TileStorage;

use crate::{main_state::MainState, terrain::TerrainBundle};

pub struct WorldGenerationPlugin;

impl Plugin for WorldGenerationPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(spawn_terrain.in_schedule(OnEnter(MainState::MapGeneration)))
            .add_system(
                main_map_generated
                    .run_if(resource_exists::<MainMap>())
                    .run_if(in_state(MainState::MapGeneration)),
            );
    }
}

#[derive(Resource)]
pub struct MainMap(Entity);

fn spawn_terrain(mut commands: Commands) {
    info!("Spawning terrain");
    let main_map = commands.spawn(TerrainBundle::default()).id();
    commands.insert_resource(MainMap(main_map));
}

fn main_map_generated(
    mut next_state: ResMut<NextState<MainState>>,
    main_map_query: Query<Entity, Added<TileStorage>>,
    main_map: Res<MainMap>,
) {
    for main_map_entity in &main_map_query {
        if main_map_entity == main_map.0 {
            info!("Main map generated");
            next_state.set(MainState::Game);
        }
    }
}
