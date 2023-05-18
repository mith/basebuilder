use std::{
    hash::{BuildHasher, Hasher},
    sync::{Arc, Mutex},
};

use ahash::{AHasher, HashMap, RandomState};
use bevy::prelude::{Component, IVec2};
use fast_poisson::Poisson2D;
use ndarray::Array2;
use noise::{NoiseFn, ScalePoint, Seedable, SuperSimplex, Turbulence};
use rand::{
    seq::{IteratorRandom, SliceRandom},
    SeedableRng,
};
use rand_xoshiro::Xoshiro256StarStar;

use crate::terrain_settings::TerrainSettings;

use super::Region;

type GeneratorFunction = Arc<Mutex<Box<dyn NoiseFn<f64, 2> + Send + Sync>>>;

#[derive(Component)]
pub(crate) struct TerrainGenerator(pub(crate) GeneratorFunction);

impl TerrainGenerator {
    pub(crate) fn new(terrain_settings: TerrainSettings) -> Self {
        let seed = terrain_settings.seed;

        let simplex = SuperSimplex::new(seed);
        let scale_point = ScalePoint::new(simplex)
            .set_scale(0.00001)
            .set_y_scale(0.1)
            .set_x_scale(0.);
        let turbulence = Turbulence::<_, SuperSimplex>::new(scale_point)
            .set_seed(seed + 1)
            .set_frequency(100.0)
            .set_power(0.01);

        let terrain_function: Arc<Mutex<Box<dyn NoiseFn<f64, 2> + Send + Sync + 'static>>> =
            Arc::new(Mutex::new(Box::new(turbulence)));
        TerrainGenerator(terrain_function)
    }
}

#[derive(Debug, Default)]
pub(crate) struct RadiusNoise {
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

pub(crate) type ChunkData = Array2<u16>;

pub(crate) fn generate_chunk(
    location: IVec2,
    regions: Arc<Mutex<HashMap<IVec2, Region>>>,
    terrain_settings: TerrainSettings,
    generator: GeneratorFunction,
) -> (IVec2, ChunkData) {
    let mut chunk_data = Array2::from_elem(
        (
            terrain_settings.chunk_size.x as usize,
            terrain_settings.chunk_size.y as usize,
        ),
        0u16,
    );

    for x in 0..terrain_settings.chunk_size.x {
        for y in 0..terrain_settings.chunk_size.y {
            let region_location = IVec2::new(location.x, location.y);

            let mut regions_guard = regions.lock().unwrap();
            let region = regions_guard.entry(region_location).or_insert_with(|| {
                let region =
                    generate_region(region_location, generator.clone(), terrain_settings.clone());
                Region { terrain: region }
            });

            chunk_data[[x as usize, y as usize]] = region.terrain[[x as usize, y as usize]];
        }
    }
    (location, chunk_data)
}

pub(crate) fn generate_region(
    region_location: IVec2,
    generator: GeneratorFunction,
    terrain_settings: TerrainSettings,
) -> Array2<u16> {
    let mut terrain = Array2::from_elem(
        (
            terrain_settings.region_size.x as usize,
            terrain_settings.region_size.y as usize,
        ),
        0u16,
    );
    let useed = terrain_settings.seed as u64;
    let mut hasher: AHasher = RandomState::with_seeds(
        useed,
        useed.swap_bytes(),
        useed.count_ones() as u64,
        useed.rotate_left(32),
    )
    .build_hasher();

    hasher.write_i32(region_location.x);
    hasher.write_i32(region_location.y);
    let ore_seed = hasher.finish();

    let mut rng = Xoshiro256StarStar::seed_from_u64(ore_seed);
    let ore_locations = Poisson2D::new()
        .with_dimensions(
            [
                terrain_settings.region_size.x as f64,
                terrain_settings.region_size.y as f64,
            ],
            5.,
        )
        .with_seed(ore_seed)
        .iter()
        .choose_multiple(&mut rng, 1);
    // .collect::<Vec<_>>();

    let ore_types = ore_locations
        .iter()
        .map(|point| {
            let ore_types = terrain_settings
                .ore_incidences
                .iter()
                .map(|(ore, inc)| (*ore, *inc))
                .collect::<Vec<_>>();

            let ore_type = ore_types
                .choose_weighted(&mut rng, |item| item.1)
                .unwrap()
                .0;

            let ore_noise = RadiusNoise {
                location: *point,
                radius: 5.,
            };

            let ore_turbulence = Turbulence::<_, SuperSimplex>::new(ore_noise)
                .set_seed((ore_seed + 1) as u32)
                .set_frequency(0.001)
                .set_power(10.);

            let ore_turbulence_function: Arc<Box<dyn NoiseFn<f64, 2> + Send + Sync>> =
                Arc::new(Box::new(ore_turbulence));

            (ore_type, ore_turbulence_function)
        })
        .collect::<Vec<_>>();

    for x in 0..terrain_settings.region_size.x as usize {
        for y in 0..terrain_settings.region_size.y as usize {
            let noise = generator.lock().unwrap().get([
                (region_location.x * terrain_settings.region_size.x as i32 + x as i32).into(),
                (region_location.y * terrain_settings.region_size.y as i32 + y as i32).into(),
            ]);

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
