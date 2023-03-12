use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use bevy_rapier2d::prelude::{Collider, RigidBody, Vect};
use ndarray::prelude::*;

use crate::{material::MaterialProperties, app_state::AppState};

#[derive(Resource)]
pub(crate) struct TerrainConfig {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) cell_size: f32,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            width: 100,
            height: 100,
            cell_size: 16.,
        }
    }
}

#[derive(Component)]
pub(crate) struct Terrain {
    materials: Array2<u16>,
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum TerrainSet {
    Update,
    Cleanup,
}

fn setup_terrain(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    config: Res<TerrainConfig>,
    material_properties: Res<MaterialProperties>,
) {
    let texture_handle = asset_server.load("textures/terrain.png");

    let tilemap_size = TilemapSize {
        x: config.width,
        y: config.height,
    };

    // create materials array with bottom half of map made of dirt, upper half empty air
    let materials = Array2::from_shape_fn((100, 100), |(_x, y)| if y < 50 { 1 } else { 0 });

    let mut tile_storage = TileStorage::empty(tilemap_size);
    let tilemap_entity = commands
        .spawn((
            Terrain {
                materials: materials.clone(),
            },
            Name::new("Terrain"),
        ))
        .id();

    {
        let origin = TilePos { x: 0, y: 0 };
        let size = TilemapSize {
            x: config.width as u32,
            y: config.height as u32,
        };
        let tilemap_id = TilemapId(tilemap_entity);
        let commands: &mut Commands = &mut commands;
        let tile_storage: &mut TileStorage = &mut tile_storage;
        for x in 0..size.x {
            for y in 0..size.y {
                let tile_material = materials[[x as usize, y as usize]];
                if tile_material == 0 {
                    continue;
                }

                let tile_pos = TilePos {
                    x: origin.x + x,
                    y: origin.y + y,
                };

                let texture_index =
                    TileTextureIndex(0);
                let tile_entity = commands
                    .spawn(TileBundle {
                        position: tile_pos,
                        tilemap_id,
                        texture_index,
                        color: material_properties.0[tile_material as usize].color.into(),
                        ..default()
                    })
                    .id();
                tile_storage.set(&tile_pos, tile_entity);
            }
        }
    };

    let tile_size = TilemapTileSize {
        x: config.cell_size,
        y: config.cell_size,
    };
    let grid_size = tile_size.into();

    let tile_colliders = build_terrain_colliders(&config, &tile_storage);

    let map_transform = Transform::from_translation(Vec3::new(
        -(config.width as f32 * config.cell_size / 2.),
        -(config.height as f32 * config.cell_size / 2.),
        0.0,
    ));

    commands.entity(tilemap_entity).insert((
        TilemapBundle {
            grid_size,
            map_type: TilemapType::Square,
            size: tilemap_size,
            storage: tile_storage,
            texture: TilemapTexture::Single(texture_handle),
            tile_size,
            transform: map_transform,
            ..default()
        },
        RigidBody::Fixed,
        Collider::compound(tile_colliders),
    ));
}

pub struct TileDamageEvent {
    pub tile: Entity,
    pub damage: u32,
}

#[derive(Component)]
pub(crate) struct TileHealth(u32);

fn update_terrain(
    mut commands: Commands,
    mut tile_damage_events: EventReader<TileDamageEvent>,
    mut damage_tiles_query: Query<&mut TileHealth>,
) {
    for damage_event in tile_damage_events.iter() {
        if let Ok(mut tile_health) = damage_tiles_query.get_mut(damage_event.tile) {
            tile_health.0 = tile_health.0.saturating_sub(damage_event.damage);
        } else {
            commands
                .entity(damage_event.tile)
                .insert(TileHealth(100u32.saturating_sub(damage_event.damage)));
        }
    }
}

fn color_damage_tile(
    mut damaged_tiles_query: Query<(&TileHealth, &mut TileColor), Changed<TileHealth>>,
) {
    for (tile_health, mut tile_color) in &mut damaged_tiles_query {
        tile_color.0 = Color::rgb(
            1.0 - (tile_health.0 as f32 / 100.0),
            tile_health.0 as f32 / 100.0,
            0.0,
        );
    }
}

pub(crate) struct TileDestroyedEvent(pub(crate) Entity);

fn remove_destroyed_tiles(
    mut commands: Commands,
    config: Res<TerrainConfig>,
    tile_query: Query<(Entity, &TileHealth, &TilePos)>,
    mut tilemap_query: Query<(Entity, &mut TileStorage), With<Terrain>>,
    mut destroyed_tiles: EventWriter<TileDestroyedEvent>,
) {
    let (tilemap_entity, mut tile_storage) = tilemap_query.single_mut();
    for (tile_entity, tile_health, tile_pos) in &tile_query {
        if tile_health.0 == 0 {
            commands.entity(tile_entity).despawn();
            tile_storage.remove(&tile_pos);
            destroyed_tiles.send(TileDestroyedEvent(tile_entity));
        }
    }

    // Rebuild tilemap collider
    let tile_colliders = build_terrain_colliders(&config, &tile_storage);

    let mut tilemap_commands = commands.entity(tilemap_entity);

    tilemap_commands.remove::<Collider>();

    if !tile_colliders.is_empty() {
        tilemap_commands.insert(Collider::compound(tile_colliders));
    }
}

fn build_terrain_colliders(
    config: &TerrainConfig,
    tile_storage: &TileStorage,
) -> Vec<(Vec2, f32, Collider)> {
    let mut tile_colliders = vec![];
    let half_cell_size = config.cell_size / 2.;
    let tilemap_size = tile_storage.size;

    for x in 0..tilemap_size.x {
        for y in 0..tilemap_size.y {
            if tile_storage.get(&TilePos { x, y }).is_none() {
                continue;
            }
            tile_colliders.push((
                Vect::new(x as f32 * config.cell_size, y as f32 * config.cell_size),
                0.,
                Collider::cuboid(half_cell_size, half_cell_size),
            ));
        }
    }
    tile_colliders
}

pub(crate) struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TerrainConfig>()
            .add_event::<TileDamageEvent>()
            .add_event::<TileDestroyedEvent>()
            .add_system(setup_terrain.in_schedule(OnEnter(AppState::Game)))
            .add_system(update_terrain.in_set(OnUpdate(AppState::Game)).in_set(TerrainSet::Update))
            .add_system(color_damage_tile.in_set(OnUpdate(AppState::Game)).in_set(TerrainSet::Update))
            .add_system(
                remove_destroyed_tiles
                    .after(TerrainSet::Update)
                    .in_set(OnUpdate(AppState::Game))
                    .in_set(TerrainSet::Cleanup),
            );
    }
}
