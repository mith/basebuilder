use bevy::{prelude::*, sprite::MaterialMesh2dBundle};
use bevy_ecs_tilemap::prelude::TilemapGridSize;

use crate::{
    climbable::Climbable,
    cursor_position::CursorPosition,
    hovered_tile::{HoveredTile, HoveredTileSet},
    structure::Structure,
    terrain::Terrain,
};

pub struct BuildPlugin;

impl Plugin for BuildPlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<BuildToolState>().add_system(
            build
                .run_if(state_exists_and_equals(BuildToolState::Placing))
                .before(HoveredTileSet),
        );
    }
}

#[derive(States, Default, Clone, Eq, PartialEq, Hash, Debug)]
pub enum BuildToolState {
    #[default]
    Inactive,
    Placing,
}

#[derive(Component)]
pub struct Ghost;

#[derive(Component)]
pub struct ContainsStructure(pub Entity);

pub const BUILDING_LAYER_Z: f32 = 2.0;

fn build(
    mut commands: Commands,
    mouse_button_input: Res<Input<MouseButton>>,
    cursor_position: Res<CursorPosition>,
    hovered_tile_query: Query<&HoveredTile>,
    terrain_query: Query<&TilemapGridSize, With<Terrain>>,
    ghost_query: Query<Entity, With<Ghost>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let tilemap_grid_size = terrain_query.single();
    // round the cursor_position to the nearest tile
    let rounded_cursor_position = Vec2::new(
        (cursor_position.0.x / tilemap_grid_size.x).round() * tilemap_grid_size.x,
        (cursor_position.0.y / tilemap_grid_size.y).round() * tilemap_grid_size.y,
    );

    // Delete all ghosts
    for ghost_entity in &ghost_query {
        commands.entity(ghost_entity).despawn_recursive();
    }

    if mouse_button_input.just_pressed(MouseButton::Left) && hovered_tile_query.is_empty() {
        commands.spawn((
            Structure,
            Climbable,
            MaterialMesh2dBundle {
                transform: Transform::from_xyz(
                    rounded_cursor_position.x,
                    rounded_cursor_position.y,
                    BUILDING_LAYER_Z,
                ),
                material: materials.add(Color::rgb(0.0, 1.0, 0.0).into()),
                mesh: meshes
                    .add(Mesh::from(shape::Quad::new(Vec2::new(
                        tilemap_grid_size.x,
                        tilemap_grid_size.y,
                    ))))
                    .into(),
                ..default()
            },
        ));
    } else {
        commands.spawn((
            Ghost,
            MaterialMesh2dBundle {
                transform: Transform::from_xyz(
                    rounded_cursor_position.x,
                    rounded_cursor_position.y,
                    BUILDING_LAYER_Z,
                ),
                material: materials.add(Color::rgba(0.0, 1.0, 0.0, 0.5).into()),
                mesh: meshes
                    .add(Mesh::from(shape::Quad::new(Vec2::new(
                        tilemap_grid_size.x,
                        tilemap_grid_size.y,
                    ))))
                    .into(),
                ..default()
            },
        ));
    }
}
