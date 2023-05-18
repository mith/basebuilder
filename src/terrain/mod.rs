mod generate;

use std::{
    hash::{BuildHasher, Hasher},
    sync::{Arc, Mutex},
};

use ahash::{HashMap, HashSet};
use bevy::{
    asset,
    math::Vec3Swizzles,
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use bevy_ecs_tilemap::prelude::*;
use bevy_rapier2d::prelude::{Collider, RigidBody, Vect};

use futures_lite::future;
use ndarray::prelude::*;

use crate::{app_state::AppState, material::MaterialProperties, terrain_settings::TerrainSettings};

use self::generate::{generate_chunk, ChunkData, TerrainGenerator};

#[derive(Component)]
pub(crate) struct Terrain;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct TerrainSet;

pub(crate) struct Region {
    terrain: Array2<u16>,
}

#[derive(Component, Reflect)]
struct Chunk(IVec2);

#[derive(Default, Resource)]
pub struct ChunkManager {
    spawned_chunks: HashSet<IVec2>,
    loading_chunks: HashSet<IVec2>,
    pub entities: HashMap<IVec2, Entity>,
    regions: Arc<Mutex<HashMap<IVec2, Region>>>,
}

#[derive(Component)]
#[component(storage = "SparseSet")]
struct GenerateChunk(pub(crate) Task<(IVec2, ChunkData)>);

fn spawn_chunks_around_camera(
    mut commands: Commands,
    camera_query: Query<&GlobalTransform, With<Camera>>,
    mut chunk_manager: ResMut<ChunkManager>,
    terrain_settings: Res<TerrainSettings>,
    generator_query: Query<&TerrainGenerator>,
) {
    for generator in &generator_query {
        for transform in &camera_query {
            let camera_chunk_pos =
                global_pos_to_chunk_pos(&transform.translation().xy(), terrain_settings.chunk_size);
            let chunk_spawn_radius = terrain_settings.chunk_spawn_radius as i32;
            for y in
                (camera_chunk_pos.y - chunk_spawn_radius)..(camera_chunk_pos.y + chunk_spawn_radius)
            {
                for x in (camera_chunk_pos.x - chunk_spawn_radius)
                    ..(camera_chunk_pos.x + chunk_spawn_radius)
                {
                    if !chunk_manager.spawned_chunks.contains(&IVec2::new(x, y))
                        && !chunk_manager.loading_chunks.contains(&IVec2::new(x, y))
                    {
                        let thread_pool = AsyncComputeTaskPool::get();
                        let regions = chunk_manager.regions.clone();
                        let terrain_settings = terrain_settings.clone();
                        let generator = Arc::clone(&generator.0);
                        let task = thread_pool.spawn(async move {
                            generate_chunk(IVec2::new(x, y), regions, terrain_settings, generator)
                        });
                        commands.spawn(GenerateChunk(task));
                        chunk_manager.loading_chunks.insert(IVec2::new(x, y));
                    }
                }
            }
        }
    }
}

pub fn global_pos_to_chunk_pos(camera_pos: &Vec2, chunk_size: UVec2) -> IVec2 {
    let camera_pos = camera_pos.as_ivec2();
    let chunk_size: IVec2 = IVec2::new(chunk_size.x as i32, chunk_size.y as i32);
    camera_pos / chunk_size
}

fn spawn_chunk(
    mut commands: Commands,
    mut chunk_manager: ResMut<ChunkManager>,
    mut generate_chunk_query: Query<(Entity, &mut GenerateChunk)>,
    terrain_settings: Res<TerrainSettings>,
    asset_server: Res<AssetServer>,
    material_properties: Res<MaterialProperties>,
) {
    for (chunk_entity, mut generate_chunk) in &mut generate_chunk_query {
        if let Some((chunk_pos, chunk_data)) =
            future::block_on(future::poll_once(&mut generate_chunk.0))
        {
            let texture_handle = asset_server.load("textures/terrain.png");
            let chunk_transform = Transform::from_translation(Vec3::new(
                chunk_pos.x as f32
                    * terrain_settings.chunk_size.x as f32
                    * terrain_settings.cell_size as f32,
                chunk_pos.y as f32
                    * terrain_settings.chunk_size.y as f32
                    * terrain_settings.cell_size as f32,
                0.0,
            ));

            let mut tile_storage = TileStorage::empty(terrain_settings.chunk_size.into());

            for ((x, y), tile) in chunk_data.indexed_iter() {
                if *tile > 0 {
                    let tile_pos = TilePos {
                        x: x as u32,
                        y: y as u32,
                    };

                    let tile_entity = commands
                        .spawn(TileBundle {
                            position: tile_pos,
                            tilemap_id: TilemapId(chunk_entity),
                            texture_index: TileTextureIndex(0),
                            color: material_properties.0[*tile as usize].color.into(),
                            ..default()
                        })
                        .id();
                    tile_storage.set(&tile_pos, tile_entity);
                }
            }

            let tile_size = TilemapTileSize {
                x: terrain_settings.cell_size as f32,
                y: terrain_settings.cell_size as f32,
            };

            let tile_colliders = build_terrain_colliders(&terrain_settings, &tile_storage);

            commands.entity(chunk_entity).insert((
                Chunk(chunk_pos),
                Name::new(format!("Chunk {:?}", chunk_pos)),
                TilemapBundle {
                    grid_size: tile_size.into(),
                    map_type: TilemapType::Square,
                    size: terrain_settings.chunk_size.into(),
                    storage: tile_storage,
                    texture: TilemapTexture::Single(texture_handle),
                    tile_size: tile_size,
                    transform: chunk_transform,
                    ..default()
                },
                RigidBody::Fixed,
                Collider::compound(tile_colliders),
            ));

            commands.entity(chunk_entity).remove::<GenerateChunk>();

            chunk_manager.loading_chunks.remove(&chunk_pos);
            chunk_manager.spawned_chunks.insert(chunk_pos);
            chunk_manager.entities.insert(chunk_pos, chunk_entity);
        }
    }
}

#[derive(Component)]
struct SetupGenerator(Task<TerrainGenerator>);

fn setup_terrain(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    config: Res<TerrainSettings>,
    material_properties: Res<MaterialProperties>,
    terrain_settings: Res<TerrainSettings>,
) {
    let settings = terrain_settings.clone();
    let thread_pool = AsyncComputeTaskPool::get();
    let task = thread_pool.spawn(async move { TerrainGenerator::new(settings) });
    commands.spawn((Terrain, SetupGenerator(task)));
}

fn spawn_generator(
    mut commands: Commands,
    mut setup_generator_query: Query<(Entity, &mut SetupGenerator)>,
) {
    for (entity, mut setup_generator) in &mut setup_generator_query {
        if let Some(generator) = future::block_on(future::poll_once(&mut setup_generator.0)) {
            commands
                .entity(entity)
                .insert(generator)
                .remove::<SetupGenerator>();
        }
    }
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
    config: Res<TerrainSettings>,
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
    config: &TerrainSettings,
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
        app.add_event::<TileDamageEvent>()
            .add_event::<TileDestroyedEvent>()
            .register_type::<Chunk>()
            .init_resource::<ChunkManager>()
            .add_system(setup_terrain.in_schedule(OnEnter(AppState::Game)))
            .add_systems(
                (spawn_generator, spawn_chunks_around_camera, spawn_chunk)
                    .in_set(OnUpdate(AppState::Game))
                    .in_set(TerrainSet),
            )
            .add_systems(
                (
                    update_terrain,
                    color_damage_tile.after(update_terrain),
                    // remove_destroyed_tiles.after(update_terrain),
                )
                    .in_set(OnUpdate(AppState::Game))
                    .in_set(TerrainSet),
            );
    }
}
