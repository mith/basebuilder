use std::hash::{BuildHasher, Hasher};

use ahash::{AHasher, RandomState};
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use bevy_rapier2d::prelude::{Collider, RigidBody, Vect};
use fast_poisson::Poisson2D;
use ndarray::prelude::*;
use noise::{NoiseFn, ScalePoint, Seedable, SuperSimplex, Turbulence};
use rand::{seq::SliceRandom, SeedableRng};
use rand_xoshiro::Xoshiro256StarStar;

use crate::{app_state::AppState, material::MaterialProperties, terrain_settings::TerrainSettings};

#[derive(Component)]
pub(crate) struct Terrain {
    materials: Array2<u16>,
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct TerrainSet;

#[derive(Debug)]
struct RadiusNoise {
    location: [f64; 2],
    radius: f64,
}

impl NoiseFn<f64, 2> for RadiusNoise {
    /// Return 1. if the point is within the radius, 0. otherwise
    fn get(&self, point: [f64; 2]) -> f64 {
        let dist = (point[0] - self.location[0]).powi(2) + (point[1] - self.location[1]).powi(2);
        if dist < self.radius.powi(2) {
            1.
        } else {
            0.
        }
    }
}

fn generate_terrain(seed: u32, terrain_settings: &TerrainSettings) -> Array2<u16> {
    let mut terrain = Array2::from_elem((100, 100), 0u16);

    let useed = seed as u64;
    let mut hasher: AHasher = RandomState::with_seeds(
        useed,
        useed.swap_bytes(),
        useed.count_ones() as u64,
        useed.rotate_left(32),
    )
    .build_hasher();
    let ore_seed = hasher.finish();

    let ore_locations = Poisson2D::new()
        .with_dimensions([100., 100.], 10.)
        .with_seed(ore_seed)
        .iter()
        .take(50)
        .collect::<Vec<_>>();

    let ore_noise = ore_locations.iter().map(|&point| RadiusNoise {
        location: point,
        radius: 5.,
    });

    let mut rng = Xoshiro256StarStar::seed_from_u64(ore_seed);
    let ore_types: Vec<(u16, _)> = ore_locations
        .iter()
        .map(|_| {
            let ore_types = terrain_settings.ore_incidences.iter().map(|(ore, inc)| {
                (*ore, *inc)
            }).collect::<Vec<_>>();

            let ore_type = ore_types
                .choose_weighted(&mut rng, |item| item.1)
                .unwrap()
                .0;
            ore_type
        })
        .zip(ore_noise)
        .collect::<Vec<_>>();

    for x in 0..100 {
        for y in 0..50 {
            let simplex = SuperSimplex::new(seed);
            let scale_point = ScalePoint::new(simplex).set_scale(0.1).set_x_scale(0.);
            let turbulence = Turbulence::<_, SuperSimplex>::new(scale_point)
                .set_seed(seed + 1)
                .set_frequency(0.001)
                .set_power(10.);
            let noise = turbulence.get([x as f64, y as f64]);

            let ore_type = ore_types.iter().fold(None, |acc, (ore_type, noise)| {
                if noise.get([x as f64, y as f64]) > 0. {
                    Some(ore_type)
                } else {
                    acc
                }
            });
            terrain[[x, y]] = if noise > 0. {
                if let Some(ore) = ore_type {
                    *ore
                } else {
                    1
                }
            } else {
                0
            };
        }
    }

    terrain
}

fn setup_terrain(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    config: Res<TerrainSettings>,
    material_properties: Res<MaterialProperties>,
) {
    let texture_handle = asset_server.load("textures/terrain.png");

    let tilemap_size = TilemapSize {
        x: config.width,
        y: config.height,
    };

    // create materials array with bottom half of map made of dirt, upper half empty air
    let materials = generate_terrain(2, &config);

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

                let texture_index = TileTextureIndex(0);
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
            .add_system(setup_terrain.in_schedule(OnEnter(AppState::Game)))
            .add_systems(
                (
                    update_terrain,
                    color_damage_tile.after(update_terrain),
                    remove_destroyed_tiles.after(update_terrain),
                )
                    .in_set(OnUpdate(AppState::Game))
                    .in_set(TerrainSet),
            );
    }
}
