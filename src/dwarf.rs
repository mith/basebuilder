use bevy::{prelude::*, sprite::MaterialMesh2dBundle};
use bevy_rapier2d::prelude::{Collider, QueryFilter, RapierContext, RigidBody};
use rand::{
    seq::{IteratorRandom, SliceRandom},
    Rng, SeedableRng,
};
use rand_xoshiro::Xoshiro256StarStar;

use crate::{
    ai_controller::AiControlled,
    app_state::AppState,
    movement::Walker,
    terrain::{TerrainSet, TerrainState},
    terrain_settings::TerrainSettings,
};

pub struct DwarfPlugin;

impl Plugin for DwarfPlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<DwarvesState>().add_system(
            spawn_dwarves
                .in_set(OnUpdate(AppState::Game))
                .run_if(in_state(TerrainState::Spawned))
                .run_if(in_state(DwarvesState::Spawning))
                .after(TerrainSet),
        );
    }
}

#[derive(States, Default, Debug, Clone, Eq, PartialEq, Hash)]
pub enum DwarvesState {
    #[default]
    Spawning,
    Spawned,
}

#[derive(Component)]
pub struct Dwarf;

fn spawn_dwarves(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    rapier_context: Res<RapierContext>,
    terrain_settings: Res<TerrainSettings>,
    mut dwarves_state: ResMut<NextState<DwarvesState>>,
) {
    let mut rng = Xoshiro256StarStar::seed_from_u64(terrain_settings.seed as u64);
    let possible_x_pos: Vec<f32> = (-5..=5)
        .map(|x| x as f32 * terrain_settings.cell_size)
        .choose_multiple(&mut rng, 6);
    for x in possible_x_pos {
        // pick location within the 10 center cells of the map
        // y location is always the top of the map
        let y = terrain_settings.cell_size * terrain_settings.height as f32 / 2.0;

        let ray_dir = Vec2::new(0.0, -1.0);
        let max_toi = terrain_settings.cell_size * terrain_settings.height as f32;
        let filter = QueryFilter::default();

        if let Some((entity, hit)) =
            rapier_context.cast_ray(Vec2::new(x, y), ray_dir, max_toi, true, filter)
        {
            commands.spawn((
                Dwarf,
                Name::new("Dwarf"),
                MaterialMesh2dBundle {
                    transform: Transform::from_xyz(x, y - hit + 6., 2.),
                    material: materials.add(Color::WHITE.into()),
                    mesh: meshes
                        .add(Mesh::from(shape::Quad::new(Vec2::new(12., 12.))))
                        .into(),
                    ..default()
                },
                RigidBody::KinematicPositionBased,
                Collider::round_cuboid(12., 12., 0.01),
                AiControlled,
                Walker::default(),
            ));
        }
    }
    dwarves_state.set(DwarvesState::Spawned);
}
