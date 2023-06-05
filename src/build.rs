use std::vec;

use bevy::{prelude::*, sprite::MaterialMesh2dBundle};
use bevy_ecs_tilemap::prelude::TilemapGridSize;
use bevy_proto::de;
use bevy_rapier2d::prelude::{Collider, CollisionGroups, Group, RapierContext};

use crate::{
    ai_controller::{ArrivedAtTargetEvent, Path},
    cursor_position::CursorPosition,
    deliver,
    dwarf::DWARF_COLLISION_GROUP,
    haul::{Haul, HaulCompletedEvent},
    hovered_tile::{HoveredTile, HoveredTileSet},
    job::{
        all_workers_eligible, job_assigned, AssignedJob, AssignedTo, AtJobSite, Commuting, Job,
        JobAssignedEvent, JobSite, StuckTimer, Worker,
    },
    pathfinding::Pathfinding,
    resource::{self, BuildingMaterial, BuildingMaterialLocator, Reserved},
    structure::Structure,
    terrain::Terrain,
};

pub struct BuildPlugin;

impl Plugin for BuildPlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<BuildToolState>()
            .add_system(
                designate_construction
                    .run_if(state_exists_and_equals(BuildToolState::Placing))
                    .before(HoveredTileSet),
            )
            .add_systems((
                designate_building_materials,
                materials_delivered,
                job_assigned::<ConstructionJob, Builder>,
                all_workers_eligible::<ConstructionJob>,
                start_building,
                build_timer,
                finish_building,
            ));
    }
}

#[derive(States, Default, Clone, Eq, PartialEq, Hash, Debug)]
pub enum BuildToolState {
    #[default]
    Inactive,
    Placing,
}

#[derive(Component)]
pub struct Ghost;

#[derive(Component)]
pub struct UnderConstruction {
    progress: u32,
}

impl Default for UnderConstruction {
    fn default() -> Self {
        Self { progress: 0 }
    }
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
    cursor_position: Res<CursorPosition>,
    hovered_tile_query: Query<&HoveredTile>,
    terrain_query: Query<&TilemapGridSize, With<Terrain>>,
    ghost_query: Query<Entity, With<Ghost>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
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

    if mouse_button_input.just_pressed(MouseButton::Left) && hovered_tile_query.is_empty() {
        commands.spawn((
            UnderConstruction::default(),
            BuildingMaterialsNeeded::new(vec![(Name::new("Log"), 1)]),
            MaterialMesh2dBundle {
                transform: Transform::from_xyz(
                    rounded_cursor_position.x,
                    rounded_cursor_position.y,
                    BUILDING_LAYER_Z,
                ),
                material: materials.add(Color::rgba(0.0, 1.0, 0.0, 0.3).into()),
                mesh: meshes
                    .add(Mesh::from(shape::Quad::new(Vec2::new(
                        tilemap_grid_size.x,
                        tilemap_grid_size.y,
                    ))))
                    .into(),
                ..default()
            },
        ));
    } else {
        commands.spawn((
            Ghost,
            MaterialMesh2dBundle {
                transform: Transform::from_xyz(
                    rounded_cursor_position.x,
                    rounded_cursor_position.y,
                    BUILDING_LAYER_Z,
                ),
                material: materials.add(Color::rgba(0.0, 1.0, 0.0, 0.5).into()),
                mesh: meshes
                    .add(Mesh::from(shape::Quad::new(Vec2::new(
                        tilemap_grid_size.x,
                        tilemap_grid_size.y,
                    ))))
                    .into(),
                ..default()
            },
        ));
    }
}

#[derive(Component)]
struct WaitingForResources;

fn designate_building_materials(
    mut commands: Commands,
    mut construction_query: Query<
        (Entity, &GlobalTransform, &mut BuildingMaterialsNeeded),
        Without<WaitingForResources>,
    >,
    building_material_locator: BuildingMaterialLocator,
    building_material_query: Query<&GlobalTransform, With<BuildingMaterial>>,
) {
    for (construction_entity, construction_transform, mut resources_needed) in
        &mut construction_query.iter()
    {
        let mut closest_resource = None;
        let mut closest_distance = f32::MAX;
        for (resource_name, amount) in resources_needed.0.iter() {
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
            let x = resource_transform.translation().x;
            let y = resource_transform.translation().y;
            let pickup_site = JobSite(vec![Vec2::new(x, y)]);
            let delivery_site = JobSite(vec![Vec2::new(
                construction_transform.translation().x,
                construction_transform.translation().y,
            )]);

            let haul_job = commands
                .spawn((
                    Job,
                    Haul::new(
                        resource_entity,
                        1,
                        construction_entity,
                        pickup_site.clone(),
                        delivery_site,
                    ),
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

#[derive(Component)]
struct ConstructionJob(Entity);

#[derive(Component, Default)]
struct Builder;

fn materials_delivered(
    mut commands: Commands,
    mut construction_query: Query<
        (&GlobalTransform, &mut BuildingMaterialsNeeded),
        With<WaitingForResources>,
    >,
    mut haul_completed_event_reader: EventReader<HaulCompletedEvent>,
    building_material_query: Query<&Name, With<BuildingMaterial>>,
) {
    for completed_haul in haul_completed_event_reader.iter() {
        if let Some((construction_entity, (construction_transform, mut resources_needed))) =
            completed_haul
                .parent_job
                .and_then(|p| construction_query.get_mut(p).ok().map(|m| (p, m)))
        {
            let Some(material_name) = building_material_query.get(completed_haul.item).ok() else {
                error!("Delivered item was not a building material");
                continue;
            };

            if let Some((_, amount_needed)) = resources_needed
                .0
                .iter_mut()
                .find(|(name, _)| name == material_name)
            {
                *amount_needed = amount_needed.saturating_sub(1);
            }
            resources_needed.0.retain(|(_, amount)| *amount > 0);

            commands.entity(completed_haul.item).despawn_recursive();

            if resources_needed.0.is_empty() {
                let delivery_site = JobSite(vec![Vec2::new(
                    construction_transform.translation().x,
                    construction_transform.translation().y,
                )]);
                commands
                    .entity(construction_entity)
                    .remove::<WaitingForResources>()
                    .with_children(|parent| {
                        parent.spawn((
                            Job,
                            ConstructionJob(construction_entity),
                            delivery_site.clone(),
                        ));
                    });
            }
        }
    }
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
    worker_query: Query<
        (Entity, &AssignedJob),
        (Without<Constructing>, With<AtJobSite>, With<Builder>),
    >,
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

fn finish_building(
    mut commands: Commands,
    construction_site_query: Query<
        (Entity, &UnderConstruction, &Handle<ColorMaterial>),
        Changed<UnderConstruction>,
    >,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for (construction_site_entity, construction_site, material_handle) in
        &mut construction_site_query.iter()
    {
        if construction_site.finished() {
            commands
                .entity(construction_site_entity)
                .remove::<UnderConstruction>()
                .insert(Structure);

            if let Some(material) = materials.get_mut(material_handle) {
                material.color = material.color.with_a(1.);
            }
        }
    }
}
