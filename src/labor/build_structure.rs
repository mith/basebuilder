use std::vec;

use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilemapGridSize;
use bevy_rapier2d::prelude::Group;

use crate::{
    building_material::{BuildingMaterial, BuildingMaterialLocator, Reserved},
    cursor_position::LastCursorPosition,
    hovered_tile::{HoveredTile, HoveredTileSet},
    labor::job::{all_workers_eligible, AssignedJob, Job, JobSite, Worker},
    ladder::spawn_ladder,
    terrain::Terrain,
};

use super::{haul::HaulRequest, job::CompletedJob, job::JobCompletedEvent};

pub struct BuildStructurePlugin;

impl Plugin for BuildStructurePlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<BuildToolState>()
            .add_event::<ConstructionCompletedEvent>()
            .register_type::<ConstructionJob>()
            .add_systems(
                Update,
                designate_construction
                    .run_if(state_exists_and_equals(BuildToolState::Placing))
                    .before(HoveredTileSet),
            )
            .add_systems(
                Update,
                (
                    designate_building_materials,
                    materials_delivered,
                    all_workers_eligible::<ConstructionJob>,
                    start_building,
                    build_timer,
                    finish_building,
                ),
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
pub struct Structure;

#[derive(Component)]
pub struct Ghost;

pub const CONSTRUCTION_COLLISION_GROUP: Group = Group::GROUP_7;

#[derive(Component, Default)]
pub struct UnderConstruction {
    progress: u32,
}

impl UnderConstruction {
    pub fn progress(&self) -> u32 {
        self.progress
    }

    pub fn add_progress(&mut self, amount: u32) {
        self.progress += amount;
        if self.progress > 100 {
            self.progress = 100;
        }
    }

    pub fn finished(&self) -> bool {
        self.progress == 100
    }
}

#[derive(Component)]
pub struct BuildingMaterialsNeeded(Vec<(Name, u32)>);

impl BuildingMaterialsNeeded {
    pub fn new(resources_needed: Vec<(Name, u32)>) -> Self {
        Self(resources_needed)
    }

    pub fn resources_needed(&self) -> &Vec<(Name, u32)> {
        &self.0
    }

    pub fn deliver_resource(&mut self, resource: &Name, amount: u32) {
        for (name, count) in &mut self.0 {
            if name == resource {
                *count = count.saturating_sub(amount);
            }
        }
    }
}

pub const BUILDING_LAYER_Z: f32 = 2.0;

fn designate_construction(
    mut commands: Commands,
    mouse_button_input: Res<Input<MouseButton>>,
    cursor_position: Res<LastCursorPosition>,
    hovered_tile_query: Query<&HoveredTile>,
    terrain_query: Query<&TilemapGridSize, With<Terrain>>,
    ghost_query: Query<Entity, With<Ghost>>,
    _materials: ResMut<Assets<ColorMaterial>>,
    _meshes: ResMut<Assets<Mesh>>,
    asset_server: Res<AssetServer>,
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

    let ladder = spawn_ladder(
        &mut commands,
        &asset_server,
        rounded_cursor_position.extend(BUILDING_LAYER_Z),
    );
    if mouse_button_input.just_pressed(MouseButton::Left) && hovered_tile_query.is_empty() {
        commands.entity(ladder).insert((
            UnderConstruction::default(),
            BuildingMaterialsNeeded::new(vec![(Name::new("Log"), 1)]),
        ));
    } else {
        commands.entity(ladder).insert(Ghost);
    }
}

#[derive(Component)]
struct WaitingForResources;

fn designate_building_materials(
    mut commands: Commands,
    construction_query: Query<
        (Entity, &GlobalTransform, &mut BuildingMaterialsNeeded),
        Without<WaitingForResources>,
    >,
    building_material_locator: BuildingMaterialLocator,
    building_material_query: Query<&GlobalTransform, With<BuildingMaterial>>,
) {
    for (construction_entity, construction_transform, resources_needed) in
        &mut construction_query.iter()
    {
        let mut closest_resource = None;
        let mut closest_distance = f32::MAX;
        for (resource_name, _amount) in resources_needed.0.iter() {
            if let Some(resource_entity) = building_material_locator
                .get_closest(resource_name, construction_transform.translation())
            {
                if let Ok(resource_transform) = building_material_query.get(resource_entity) {
                    let distance = construction_transform
                        .translation()
                        .distance(resource_transform.translation());
                    if distance < closest_distance {
                        closest_distance = distance;
                        closest_resource = Some(resource_entity);
                    }
                }
            }
        }

        if let Some(resource_entity) = closest_resource {
            let resource_transform = building_material_query
                .get(resource_entity)
                .expect("Resource entity should have a transform");
            let pickup_site = {
                let x = resource_transform.translation().x;
                let y = resource_transform.translation().y;
                JobSite(vec![Vec2::new(x, y)])
            };
            let haul_job = commands
                .spawn((
                    Job,
                    HaulRequest::request_entity(resource_entity, construction_entity),
                    pickup_site,
                ))
                .id();

            commands
                .entity(construction_entity)
                .insert(WaitingForResources)
                .add_child(haul_job);

            commands.entity(resource_entity).insert(Reserved);
        }
    }
}

#[derive(Component, Debug, Clone, Reflect)]
struct ConstructionJob(Entity);

#[derive(Component, Default, Debug)]
struct Builder;

fn materials_delivered(
    _commands: Commands,
    _construction_query: Query<
        (&GlobalTransform, &mut BuildingMaterialsNeeded),
        With<WaitingForResources>,
    >,
    mut job_completed_event_reader: EventReader<JobCompletedEvent>,
    _building_material_query: Query<&Name, With<BuildingMaterial>>,
) {
    for _job in job_completed_event_reader.iter() {}
}

#[derive(Component)]
struct Constructing(Entity);

#[derive(Component)]
struct BuildTimer(Timer);

impl Default for BuildTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(1.0, TimerMode::Repeating))
    }
}

fn start_building(
    mut commands: Commands,
    worker_query: Query<(Entity, &AssignedJob), (Without<Constructing>, With<Builder>)>,
    construction_job_query: Query<&ConstructionJob>,
) {
    for (worker_entity, assigned_job) in &worker_query {
        if let Ok(construction_entity) = construction_job_query.get(assigned_job.0).map(|job| job.0)
        {
            commands
                .entity(worker_entity)
                .insert((BuildTimer::default(), Constructing(construction_entity)));
        }
    }
}

fn build_timer(
    time: Res<Time>,
    mut constructing_worker_query: Query<
        (&mut BuildTimer, &AssignedJob),
        (With<Worker>, With<Constructing>),
    >,
    construction_job_query: Query<&ConstructionJob>,
    mut construction_site_query: Query<&mut UnderConstruction>,
) {
    for (mut timer, job) in &mut constructing_worker_query {
        if timer.0.tick(time.delta()).just_finished() {
            if let Ok(mut construction_site) = construction_job_query
                .get(job.0)
                .and_then(|cj| construction_site_query.get_mut(cj.0))
            {
                construction_site.add_progress(20);
            }
        }
    }
}

#[derive(Event)]
pub struct ConstructionCompletedEvent {
    pub construction_site: Entity,
}

fn finish_building(
    mut commands: Commands,
    construction_site_query: Query<(Entity, &UnderConstruction), Changed<UnderConstruction>>,
    mut construction_completed_event_writer: EventWriter<ConstructionCompletedEvent>,
    construction_job_query: Query<&ConstructionJob>,
) {
    for (construction_site_entity, construction_site) in &mut construction_site_query.iter() {
        if construction_site.finished() {
            commands
                .entity(construction_site_entity)
                .remove::<UnderConstruction>()
                .insert(Structure);

            if let Some(construction_job) = construction_job_query
                .iter()
                .find(|cj| cj.0 == construction_site_entity)
            {
                commands.entity(construction_job.0).insert(CompletedJob);
            }

            construction_completed_event_writer.send(ConstructionCompletedEvent {
                construction_site: construction_site_entity,
            });
        }
    }
}
