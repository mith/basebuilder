use bevy::{prelude::*, utils::HashMap};
use bevy_proto::prelude::{ProtoCommands, PrototypesMut};

pub struct StructurePlugin;

impl Plugin for StructurePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<GhostSpawnEvent>()
            .add_event::<StructureSpawnEvent>()
            .add_state::<StructureState>()
            .init_resource::<SpawnedStructures>()
            .add_system(load.in_schedule(OnEnter(StructureState::Loading)))
            .add_system(loading.run_if(in_state(StructureState::Loading)))
            .add_system(spawn_ghost_structure.run_if(in_state(StructureState::Loaded)));
    }
}

#[derive(States, Default, Debug, Clone, Eq, PartialEq, Hash)]
pub enum StructureState {
    #[default]
    Loading,
    Loaded,
}

#[derive(Resource)]
struct StructuresHandles(Vec<HandleUntyped>);

fn load(mut commands: Commands, mut prototypes: PrototypesMut) {
    let structures_handle = prototypes
        .load_folder("structures")
        .expect("Failed to load structures");

    commands.insert_resource(StructuresHandles(structures_handle));
}

fn loading(
    asset_server: Res<AssetServer>,
    structures_handles: Res<StructuresHandles>,
    mut state: ResMut<NextState<StructureState>>,
) {
    match asset_server.get_group_load_state(structures_handles.0.iter().map(|handle| handle.id())) {
        bevy::asset::LoadState::Loaded => {
            state.set(StructureState::Loaded);
        }
        _ => {}
    }
}

#[derive(Component, Debug, Clone, Eq, PartialEq, Hash)]
pub struct Structure;

#[derive(Component, Debug, Clone, Eq, PartialEq, Hash)]
pub struct Ghost;

#[derive(Resource, Default)]
struct Structures(Vec<String>);

#[derive(Resource, Default)]
struct SpawnedStructures(HashMap<String, Entity>);

pub struct GhostSpawnEvent {
    pub name: String,
    pub position: Vec2,
}
pub struct StructureSpawnEvent {
    pub name: String,
    pub position: Vec2,
}

fn spawn_ghost_structure(
    mut commands: ProtoCommands,
    mut spawned_ghosts: ResMut<SpawnedStructures>,
    mut ghost_spawn_events: EventReader<GhostSpawnEvent>,
) {
    for event in ghost_spawn_events.iter() {
        let spawned_ghost = commands
            .spawn(event.name.clone())
            .entity_commands()
            .insert(Ghost)
            .id();
        spawned_ghosts.0.insert(event.name.clone(), spawned_ghost);
    }
}
