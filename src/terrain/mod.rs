mod terrain_params;

use std::sync::Arc;

use bevy::{prelude::*, sprite::MaterialMesh2dBundle};

use bevy_ecs_tilemap::prelude::*;
use bevy_rapier2d::prelude::{Collider, CollisionGroups, Group, RigidBody, Vect};

#[cfg(feature = "async")]
use {
    bevy::tasks::{AsyncComputeTaskPool, Task},
    futures_lite::future,
};

use ndarray::prelude::*;

use crate::{
    main_state::MainState, material::MaterialProperties, terrain_settings::TerrainSettings,
};

use terrain_gen::{create_terrain_generator_function, generate_terrain, GeneratorFunction};

pub use self::terrain_params::TerrainParam;
pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<TileDamageEvent>()
            .add_event::<TileDestroyedEvent>()
            .add_systems(
                Update,
                (setup_terrain, spawn_tilemap).run_if(resource_exists::<TerrainSettings>()),
            )
            .add_systems(
                Update,
                spawn_tilemap
                    .run_if(in_state(MainState::Game))
                    .in_set(TerrainSet),
            )
            .add_systems(
                Update,
                (update_terrain, apply_deferred)
                    .chain()
                    .run_if(in_state(MainState::Game))
                    .in_set(TerrainSet)
                    .in_set(TerrainUpdateSet),
            )
            .add_systems(
                Update,
                (color_damage_tile, remove_destroyed_tiles)
                    .run_if(in_state(MainState::Game))
                    .in_set(TerrainSet)
                    .after(TerrainUpdateSet),
            );

        #[cfg(feature = "async")]
        app.add_systems(
            Update,
            spawn_terrain_data.run_if(resource_exists::<TerrainSettings>()),
        );
    }
}

#[derive(Component, Default)]
pub struct Terrain;

#[derive(Bundle, Default)]
pub struct TerrainBundle {
    pub terrain: Terrain,
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct TerrainSet;

#[derive(Component)]
pub struct TerrainData(Array2<u16>);

impl TerrainData {
    pub fn get_tile(&self, tile_pos: UVec2) -> Option<u16> {
        self.0
            .get([tile_pos.x as usize, tile_pos.y as usize])
            .copied()
    }

    pub fn map_size(&self) -> UVec2 {
        let shape = self.0.shape();
        UVec2::new(shape[0] as u32, shape[1] as u32)
    }
}
#[derive(Component)]
pub struct TerrainGenerator(pub GeneratorFunction);

impl TerrainGenerator {
    pub fn new(terrain_settings: TerrainSettings) -> Self {
        Self(create_terrain_generator_function(terrain_settings.into()))
    }
}

#[cfg(feature = "async")]
#[derive(Component)]
#[component(storage = "SparseSet")]
struct GenerateTerrain(pub Task<TerrainData>);

#[cfg(feature = "async")]
#[derive(Component)]
struct SetupGenerator(Task<TerrainGenerator>);

fn setup_terrain(
    mut commands: Commands,
    terrain_settings: Res<TerrainSettings>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    new_terrain_query: Query<Entity, Added<Terrain>>,
) {
    for terrain_entity in &new_terrain_query {
        info!("Setting up terrain");
        let settings = terrain_settings.clone();
        let generator = TerrainGenerator::new(settings);

        #[cfg(feature = "async")]
        {
            info!("Generating terrain asynchronously");
            let task = generate_terrain_async(generator, terrain_settings.clone().into());
            commands.spawn(GenerateTerrain(task));
        }

        #[cfg(not(feature = "async"))]
        {
            info!("Generating terrain synchronously");
            let generator = Arc::clone(&generator.0);
            let terrain_data =
                generate_terrain(IVec2::new(0, 0), generator, terrain_settings.clone().into());
            commands
                .entity(terrain_entity)
                .insert(TerrainData(terrain_data));
        }

        // Spawn a quad behind the terrain to act as a background
        commands.spawn((
            Name::new("Background"),
            MaterialMesh2dBundle {
                transform: Transform::from_xyz(
                    -terrain_settings.cell_size / 2. + 0.5 * terrain_settings.cell_size,
                    -terrain_settings.cell_size / 2. + 0.5 * terrain_settings.cell_size,
                    TERRAIN_LAYER_Z,
                ),
                material: materials.add(Color::TEAL.into()),
                mesh: meshes
                    .add(Mesh::from(shape::Quad::new(Vec2::new(
                        terrain_settings.cell_size * terrain_settings.width as f32,
                        terrain_settings.cell_size * terrain_settings.height as f32,
                    ))))
                    .into(),
                ..default()
            },
        ));
    }
}

#[cfg(feature = "async")]
fn generate_terrain_async(
    generator: TerrainGenerator,
    terrain_settings: TerrainSettings,
) -> Task<TerrainData> {
    let thread_pool = AsyncComputeTaskPool::get();
    let terrain_settings_clone = terrain_settings.clone();
    let task = thread_pool.spawn(async move {
        let region = generate_terrain(IVec2::new(0, 0), generator, terrain_settings);
        TerrainData(region)
    });
    task
}

#[cfg(feature = "async")]
fn spawn_terrain_data(
    mut commands: Commands,
    mut generate_terrain_query: Query<(Entity, &mut GenerateTerrain)>,
) {
    for (terrain_entity, mut generate_terrain) in &mut generate_terrain_query {
        if let Some(terrain_data) = future::block_on(future::poll_once(&mut generate_terrain.0)) {
            commands
                .entity(terrain_entity)
                .remove::<GenerateTerrain>()
                .insert(terrain_data);
        }
    }
}

pub const TERRAIN_LAYER_Z: f32 = 0.0;
pub const TERRAIN_COLLISION_GROUP: Group = Group::GROUP_1;

fn spawn_tilemap(
    mut commands: Commands,
    mut new_terrain_data_query: Query<(Entity, &TerrainData), Added<TerrainData>>,
    terrain_settings: Res<TerrainSettings>,
    asset_server: Res<AssetServer>,
    material_properties: Res<MaterialProperties>,
) {
    for (terrain_entity, terrain_data) in &mut new_terrain_data_query {
        let texture_handle = asset_server.load("textures/terrain.png");
        let tilemap_size = TilemapSize {
            x: terrain_settings.width,
            y: terrain_settings.height,
        };
        let mut tile_storage = TileStorage::empty(tilemap_size);

        let terrain_transform = Transform::from_translation(Vec3::new(
            -(terrain_settings.width as f32 * terrain_settings.cell_size) / 2.0
                + 0.5 * terrain_settings.cell_size,
            -(terrain_settings.height as f32 * terrain_settings.cell_size) / 2.0
                + 0.5 * terrain_settings.cell_size,
            TERRAIN_LAYER_Z,
        ));

        for ((x, y), tile) in terrain_data.0.indexed_iter() {
            if *tile > 0 {
                let tile_pos = TilePos {
                    x: x as u32,
                    y: y as u32,
                };
                commands.entity(terrain_entity).with_children(|parent| {
                    let tile_entity = parent
                        .spawn((
                            Name::new("TerrainTile"),
                            TileBundle {
                                position: tile_pos,
                                tilemap_id: TilemapId(terrain_entity),
                                texture_index: TileTextureIndex(0),
                                color: material_properties.0[*tile as usize].color.into(),
                                ..default()
                            },
                            TransformBundle::from_transform(Transform::from_translation(
                                Vec3::new(
                                    x as f32 * terrain_settings.cell_size,
                                    y as f32 * terrain_settings.cell_size,
                                    0.0,
                                ),
                            )),
                        ))
                        .id();
                    tile_storage.set(&tile_pos, tile_entity);
                });
            }
        }

        let tile_size = TilemapTileSize {
            x: terrain_settings.cell_size,
            y: terrain_settings.cell_size,
        };

        let tile_colliders = build_terrain_colliders(&terrain_settings, &tile_storage);

        commands.entity(terrain_entity).insert((
            Name::new("Terrain"),
            TilemapBundle {
                grid_size: tile_size.into(),
                map_type: TilemapType::Square,
                size: tilemap_size,
                storage: tile_storage,
                texture: TilemapTexture::Single(texture_handle),
                tile_size,
                transform: terrain_transform,
                ..default()
            },
            RigidBody::Fixed,
            CollisionGroups::new(TERRAIN_COLLISION_GROUP, Group::ALL),
            Collider::compound(tile_colliders),
        ));
    }
}

#[derive(Event)]
pub struct TileDamageEvent {
    pub tile: Entity,
    pub damage: u32,
}

#[derive(Component)]
pub struct TileHealth(u32);

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
struct TerrainUpdateSet;

fn update_terrain(
    mut commands: Commands,
    mut tile_damage_events: EventReader<TileDamageEvent>,
    mut damage_tiles_query: Query<&mut TileHealth>,
) {
    for damage_event in tile_damage_events.iter() {
        if let Ok(mut tile_health) = damage_tiles_query.get_mut(damage_event.tile) {
            tile_health.0 = tile_health.0.saturating_sub(damage_event.damage);
        } else if let Some(mut tile_entity) = commands.get_entity(damage_event.tile) {
            tile_entity.insert(TileHealth(100u32.saturating_sub(damage_event.damage)));
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

#[derive(Event)]
pub struct TileDestroyedEvent {
    pub entity: Entity,
    pub tile_pos: TilePos,
}

fn remove_destroyed_tiles(
    mut commands: Commands,
    config: Res<TerrainSettings>,
    tile_query: Query<(Entity, &TileHealth, &TilePos), Changed<TileHealth>>,
    mut tilemap_query: Query<(Entity, &mut TileStorage, &mut TerrainData), With<Terrain>>,
    mut destroyed_tiles: EventWriter<TileDestroyedEvent>,
) {
    let (tilemap_entity, mut tile_storage, mut terrain_data) = tilemap_query.single_mut();
    for (tile_entity, tile_health, tile_pos) in &tile_query {
        if tile_health.0 == 0 {
            commands.entity(tile_entity).despawn_recursive();
            tile_storage.remove(tile_pos);
            terrain_data.0[[tile_pos.x as usize, tile_pos.y as usize]] = 0;
            destroyed_tiles.send(TileDestroyedEvent {
                entity: tile_entity,
                tile_pos: *tile_pos,
            });
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
