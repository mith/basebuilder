use std::{
    hash::{BuildHasher, Hasher},
    sync::{Arc, Mutex},
};

use ahash::{AHasher, RandomState};
use fast_poisson::Poisson2D;
use glam::IVec2;
use hashbrown::HashMap;
use ndarray::Array2;
use noise::{NoiseFn, Seedable, SuperSimplex, TranslatePoint, Turbulence};
use rand::{
    seq::{IteratorRandom, SliceRandom},
    SeedableRng,
};
use rand_xoshiro::Xoshiro256StarStar;

#[derive(Clone, Debug)]
pub struct TerrainGeneratorSettings {
    pub width: u32,
    pub height: u32,
    pub cell_size: f32,
    pub ore_incidences: HashMap<u16, f32>,
    pub seed: u32,
}
pub type GeneratorFunction = Arc<Mutex<Box<dyn NoiseFn<f64, 2> + Send + Sync>>>;

pub fn create_terrain_generator_function(
    generator_settings: TerrainGeneratorSettings,
) -> GeneratorFunction {
    let seed = generator_settings.seed;

    // let simplex = SuperSimplex::new(seed);
    // let scale_point = ScalePoint::new(simplex)
    //     .set_scale(0.05); //.set_x_scale(0.);
    let plane = PlaneNoise { height: 0. };
    let turbulence = Turbulence::<_, SuperSimplex>::new(plane)
        .set_seed(seed)
        .set_frequency(0.003)
        .set_power(10.0);

    let translate = TranslatePoint::new(turbulence)
        // .set_x_translation(terrain_settings.width as f64 / 2.)
        .set_y_translation(-(generator_settings.height as f64) / 2.);

    let terrain_function: Arc<Mutex<Box<dyn NoiseFn<f64, 2> + Send + Sync + 'static>>> =
        Arc::new(Mutex::new(Box::new(translate)));
    terrain_function
}

#[derive(Debug, Default)]
pub struct RadiusNoise {
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

#[derive(Debug, Default)]
pub struct PlaneNoise {
    height: f64,
}

impl NoiseFn<f64, 2> for PlaneNoise {
    fn get(&self, point: [f64; 2]) -> f64 {
        if point[1] < self.height {
            1.
        } else {
            0.
        }
    }
}

pub fn generate_terrain(
    region_location: IVec2,
    generator: GeneratorFunction,
    terrain_settings: TerrainGeneratorSettings,
) -> Array2<u16> {
    let mut terrain = Array2::from_elem(
        (
            terrain_settings.width as usize,
            terrain_settings.height as usize,
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
                terrain_settings.width as f64,
                terrain_settings.height as f64,
            ],
            5.,
        )
        .with_seed(ore_seed)
        .iter()
        .choose_multiple(&mut rng, 100);

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

    for x in 0..terrain_settings.width as usize {
        for y in 0..terrain_settings.height as usize {
            let noise = generator.lock().unwrap().get([
                (region_location.x * terrain_settings.width as i32 + x as i32).into(),
                (region_location.y * terrain_settings.height as i32 + y as i32).into(),
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
