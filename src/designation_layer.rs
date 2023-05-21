use bevy::prelude::*;
use bevy_ecs_tilemap::{
    prelude::*,
    tiles::{TileColor, TilePos, TileStorage},
    TilemapBundle,
};

use crate::{app_state::AppState, terrain::TileDestroyedEvent, terrain_settings::TerrainSettings};

#[derive(Component)]
struct HoverLayer;

fn setup_designation_layer(
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
        1.0,
    ));
    commands.spawn((
        HoverLayer,
        Name::new("Designated Tile layer"),
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

#[derive(Component)]
pub(crate) struct Designated;

const HIGHLIGHT_COLOR: Color = Color::rgba(1., 1., 0.2, 0.2);

fn highlight_designation_tile(
    mut commands: Commands,
    mut tile_query: Query<&TilePos, Added<Designated>>,
    mut tilemap_query: Query<(Entity, &mut TileStorage), With<HoverLayer>>,
) {
    for (tilemap_entity, mut tile_storage) in &mut tilemap_query {
        for tile_pos in &mut tile_query {
            let tile_entity = commands
                .spawn(TileBundle {
                    position: *tile_pos,
                    tilemap_id: TilemapId(tilemap_entity),
                    texture_index: TileTextureIndex(0),
                    color: TileColor(HIGHLIGHT_COLOR),
                    ..default()
                })
                .id();
            tile_storage.set(tile_pos, tile_entity);
        }
    }
}

fn unhighlight_designation_tile(
    mut commands: Commands,
    mut designation_tiles_removed: RemovedComponents<Designated>,
    tile_query: Query<&TilePos>,
    mut tilemap_query: Query<&mut TileStorage, With<HoverLayer>>,
    mut destroyed_tiles: EventReader<TileDestroyedEvent>,
) {
    let mut tile_storage = tilemap_query.single_mut();
    for undesignation_tile_entity in designation_tiles_removed.iter() {
        if let Ok(tile_pos) = tile_query.get(undesignation_tile_entity) {
            if let Some(hover_tile_entity) = tile_storage.get(tile_pos) {
                commands.entity(hover_tile_entity).despawn_recursive();
                tile_storage.remove(&tile_pos);
            }
        }
    }

    for destroyed_tile in destroyed_tiles.iter() {
        if let Some(hover_tile_entity) = tile_storage.get(&destroyed_tile.tile_pos) {
            commands.entity(hover_tile_entity).despawn_recursive();
            tile_storage.remove(&destroyed_tile.tile_pos);
        }
    }
}

#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Debug)]
pub(crate) struct DesignationLayerSet;

pub(crate) struct DesignationLayerPlugin;

impl Plugin for DesignationLayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(setup_designation_layer.in_schedule(OnEnter(AppState::Game)))
            .add_systems(
                (
                    apply_system_buffers,
                    highlight_designation_tile,
                    unhighlight_designation_tile,
                )
                    .chain()
                    .in_set(OnUpdate(AppState::Game))
                    .in_set(DesignationLayerSet),
            );
    }
}
