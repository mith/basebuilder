use bevy::{math::Vec3Swizzles, prelude::*};
use bevy_ecs_tilemap::{
    prelude::*,
    tiles::{TileColor, TilePos, TileStorage},
    TilemapBundle,
};

use crate::{
    app_state::AppState,
    cursor_position::CursorPosition,
    terrain::{TerrainParams, TerrainSet, TileDestroyedEvent},
    terrain_settings::TerrainSettings,
};

#[derive(Component)]
struct HoverLayer;

fn setup_hovered_layer(
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
        Name::new("Hovered Tile layer"),
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
pub(crate) struct HoveredTile;

fn hovered_tile(
    mut commands: Commands,
    cursor_pos: Res<CursorPosition>,
    hovered_tiles_query: Query<Entity, With<HoveredTile>>,
    terrain: TerrainParams,
) {
    let cursor_tile_pos = terrain.global_to_tile_pos(cursor_pos.0.xy());

    if let Some(tile_entity) =
        cursor_tile_pos.and_then(|tile_pos| terrain.get_tile_entity(tile_pos))
    {
        commands.entity(tile_entity).insert(HoveredTile);
        for hovered_tile in &mut hovered_tiles_query.iter() {
            if hovered_tile != tile_entity {
                commands.entity(hovered_tile).remove::<HoveredTile>();
            }
        }
    } else {
        for hovered_tile in &mut hovered_tiles_query.iter() {
            commands.entity(hovered_tile).remove::<HoveredTile>();
        }
    }
}

const HIGHLIGHT_COLOR: Color = Color::rgba(1., 1., 0.2, 0.2);

fn highlight_hovered_tile(
    mut commands: Commands,
    mut tile_query: Query<&TilePos, Added<HoveredTile>>,
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

fn unhighlight_hovered_tile(
    mut commands: Commands,
    mut hovered_tiles_removed: RemovedComponents<HoveredTile>,
    tile_query: Query<&TilePos>,
    mut hoverlayer_tilemap_query: Query<&mut TileStorage, With<HoverLayer>>,
    mut destroyed_tiles: EventReader<TileDestroyedEvent>,
) {
    let mut hovertile_storage = hoverlayer_tilemap_query.single_mut();
    for unhovered_tile_entity in hovered_tiles_removed.iter() {
        if let Ok(tile_pos) = tile_query.get(unhovered_tile_entity) {
            if let Some(hover_tile_entity) = hovertile_storage.get(tile_pos) {
                commands.entity(hover_tile_entity).despawn_recursive();
                hovertile_storage.remove(&tile_pos);
            }
        } // else: tile was destroyed and tile_pos can no longer be queried
    }
    // remove destroyed tiles from hoverlayer
    for destroyed_tile in destroyed_tiles.iter() {
        if let Some(hovertile_entity) = hovertile_storage.get(&destroyed_tile.tile_pos) {
            commands.entity(hovertile_entity).despawn_recursive();
            hovertile_storage.remove(&destroyed_tile.tile_pos);
        }
    }
}

#[derive(SystemSet, Hash, PartialEq, Eq, Clone, Debug)]
pub(crate) struct HoveredTileSet;

pub(crate) struct HoveredTilePlugin;

impl Plugin for HoveredTilePlugin {
    fn build(&self, app: &mut App) {
        app.add_system(setup_hovered_layer.in_schedule(OnEnter(AppState::Game)))
            .add_systems(
                (
                    hovered_tile,
                    apply_system_buffers,
                    highlight_hovered_tile,
                    unhighlight_hovered_tile,
                )
                    .chain()
                    .in_set(OnUpdate(AppState::Game))
                    .in_set(HoveredTileSet)
                    .after(TerrainSet),
            );
    }
}
