use bevy::{prelude::*, sprite::MaterialMesh2dBundle};

use crate::{ai_controller::Path, terrain::TerrainParams};

use super::DebugSet;

pub struct PathfindingDebugPlugin;

impl Plugin for PathfindingDebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<PathfindingDebugState>()
            .add_systems(
                Update,
                (render_paths, despawn_removed_paths)
                    .run_if(in_state(PathfindingDebugState::Enabled))
                    .in_set(DebugSet),
            )
            .add_systems(OnExit(PathfindingDebugState::Enabled), despawn_debug_nodes);
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
