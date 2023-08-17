use bevy::{prelude::*, sprite::MaterialMesh2dBundle};
use bevy_ecs_tilemap::{
    prelude::{TilemapId, TilemapSize, TilemapTexture, TilemapTileSize, TilemapType},
    tiles::{TileBundle, TileColor, TilePos, TileStorage, TileTextureIndex},
    TilemapBundle,
};

use crate::{
    main_state::MainState,
    pathfinding::{can_stand, Path},
    terrain::{Terrain, TerrainParams},
    terrain_settings::TerrainSettings,
};

use super::DebugSet;

pub struct PathfindingDebugPlugin;

impl Plugin for PathfindingDebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<PathfindingDebugState>()
            .add_systems(
                Update,
                (
                    render_paths,
                    despawn_removed_paths,
                    highlight_walkable_tiles,
                )
                    .run_if(in_state(PathfindingDebugState::Enabled))
                    .in_set(DebugSet),
            )
            .add_systems(OnEnter(MainState::Game), setup_walkable_layer)
            .add_systems(OnExit(PathfindingDebugState::Enabled), despawn_debug_nodes);
    }
}

#[derive(Component)]
struct WalkableLayer;

fn setup_walkable_layer(
    mut commands: Commands,
    config: Res<TerrainSettings>,
    asset_server: Res<AssetServer>,
) {
    let texture_handle = asset_server.load("textures/terrain.png");

    let tile_size = TilemapTileSize {
        x: config.cell_size,
        y: config.cell_size,
    };

    let grid_size = tile_size.into();

    let tilemap_size = TilemapSize {
        x: config.width,
        y: config.height,
    };

    let storage = TileStorage::empty(tilemap_size);

    let map_transform = Transform::from_translation(Vec3::new(
        -(config.width as f32 * config.cell_size / 2.),
        -(config.height as f32 * config.cell_size / 2.),
        20.,
    ));

    commands.spawn((
        Name::new("Walkable Layer"),
        WalkableLayer,
        TilemapBundle {
            grid_size,
            map_type: TilemapType::Square,
            size: tilemap_size,
            storage,
            texture: TilemapTexture::Single(texture_handle),
            tile_size,
            transform: map_transform,
            ..default()
        },
    ));
}

fn highlight_walkable_tiles(
    mut commands: Commands,
    mut tilemap_query: Query<(Entity, &mut TileStorage), (With<WalkableLayer>, Without<Terrain>)>,
    config: Res<TerrainSettings>,
    terrain: TerrainParams,
) {
    let terrain_data = terrain.terrain_data_query.single();

    const WALKABLE_COLOR: Color = Color::rgba(0., 1., 0., 0.3);

    // for each tile, check if it's walkable and set the tile to the correct color
    for (tilemap_entity, mut storage) in &mut tilemap_query {
        for x in 0..config.width {
            for y in 0..config.height {
                let tile_pos = TilePos::new(x, y);
                let walkable = can_stand(terrain_data, tile_pos);

                if walkable && storage.get(&tile_pos).is_none() {
                    let tile_entity = commands
                        .spawn((
                            Name::new("Walkable tile"),
                            TileBundle {
                                position: tile_pos,
                                tilemap_id: TilemapId(tilemap_entity),
                                texture_index: TileTextureIndex(0),
                                color: TileColor(WALKABLE_COLOR),
                                ..default()
                            },
                        ))
                        .id();

                    storage.set(&tile_pos, tile_entity);
                }

                if !walkable {
                    if let Some(tile_entity) = storage.get(&tile_pos) {
                        commands.entity(tile_entity).despawn_recursive();
                        storage.remove(&tile_pos);
                    }
                }
            }
        }
    }
}

#[derive(States, Default, Debug, Clone, Eq, PartialEq, Hash)]
pub enum PathfindingDebugState {
    #[default]
    Disabled,
    Enabled,
}

#[derive(Component)]
pub struct PathfindingDebugNode(Entity);

#[derive(Component)]
pub struct PathfindingDebugNodes(Vec<Entity>);

fn render_paths(
    mut commands: Commands,
    paths: Query<(Entity, &Path, Option<&PathfindingDebugNodes>), Changed<Path>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    terrain: TerrainParams,
) {
    for (path_entity, path, opt_debug_nodes) in &paths {
        if let Some(debug_nodes) = opt_debug_nodes {
            for &node in &debug_nodes.0 {
                if let Some(entity_commands) = commands.get_entity(node) {
                    entity_commands.despawn_recursive();
                }
            }
        }
        let mut debug_nodes = Vec::new();
        for node in path.0.iter() {
            let node_position = terrain.tile_to_global_pos((*node).into());
            let debug_node_id = commands
                .spawn((
                    Name::new("Pathfinding debug node"),
                    PathfindingDebugNode(path_entity),
                    MaterialMesh2dBundle {
                        mesh: meshes.add(Mesh::from(shape::Circle::new(6.))).into(),
                        material: materials.add(Color::rgb(0.0, 1.0, 0.0).into()),
                        transform: Transform::from_translation(node_position.extend(4.)),
                        ..default()
                    },
                ))
                .id();
            debug_nodes.push(debug_node_id);
        }
        commands
            .entity(path_entity)
            .insert(PathfindingDebugNodes(debug_nodes));
    }
}

fn despawn_removed_paths(
    mut commands: Commands,
    mut removed_paths: RemovedComponents<Path>,
    pathfinding_debug_nodes: Query<(Entity, &PathfindingDebugNode)>,
) {
    for path_entity in &mut removed_paths {
        for (debug_node_entity, debug_node) in &pathfinding_debug_nodes {
            if debug_node.0 == path_entity {
                commands.entity(debug_node_entity).despawn_recursive()
            }
        }
        commands
            .entity(path_entity)
            .remove::<PathfindingDebugNode>();
    }
}

fn despawn_debug_nodes(
    mut commands: Commands,
    mut pathfinding_debug_nodes: Query<(Entity, &PathfindingDebugNodes)>,
) {
    for (path_entity, debug_nodes) in &mut pathfinding_debug_nodes {
        for node in debug_nodes.0.iter() {
            commands.entity(*node).despawn_recursive()
        }
        commands
            .entity(path_entity)
            .remove::<PathfindingDebugNodes>();
    }
}
