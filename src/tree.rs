use bevy::{prelude::*, sprite::MaterialMesh2dBundle};
use bevy_rapier2d::prelude::{
    Collider, CollisionGroups, Group, QueryFilter, RapierContext, RigidBody,
};
use rand::{seq::IteratorRandom, SeedableRng};
use rand_xoshiro::Xoshiro256StarStar;

use crate::{
    health::Health,
    labor::chop_tree::PICKER_COLLISION_GROUP,
    resource::BuildingMaterial,
    terrain::{TerrainSet, TerrainState, TERRAIN_COLLISION_GROUP},
    terrain_settings::TerrainSettings,
};

pub struct TreePlugin;

impl Plugin for TreePlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<TreesState>()
            .add_event::<TreeDamageEvent>()
            .add_event::<TreeDestroyedEvent>()
            .add_system(
                spawn_trees
                    .in_schedule(OnEnter(TerrainState::Spawned))
                    .after(TerrainSet),
            )
            .add_systems((update_tree_health, destroy_trees));
    }
}

#[derive(States, Default, Debug, Clone, Eq, PartialEq, Hash)]
pub enum TreesState {
    #[default]
    Spawning,
    Spawned,
}

fn spawn_trees(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    terrain_settings: Res<TerrainSettings>,
    rapier_context: Res<RapierContext>,
    mut trees_state: ResMut<NextState<TreesState>>,
) {
    let mut rng = Xoshiro256StarStar::seed_from_u64(terrain_settings.seed as u64);
    let terrain_half_width = terrain_settings.width as f32 / 2.0 / 3.;
    let possible_x_pos: Vec<f32> = (-terrain_half_width as i32..=terrain_half_width as i32)
        .filter(|x| (*x).abs() > 5)
        .map(|x| x as f32 * 3. * terrain_settings.cell_size)
        .choose_multiple(&mut rng, 25);
    for x in possible_x_pos {
        // pick location within the 10 center cells of the map
        // y location is always the top of the map
        let y = terrain_settings.cell_size * terrain_settings.height as f32 / 2.0;

        let ray_dir = Vec2::new(0.0, -1.0);
        let max_toi = terrain_settings.cell_size * terrain_settings.height as f32;
        let filter = QueryFilter::default();

        if let Some((_entity, hit)) =
            rapier_context.cast_ray(Vec2::new(x, y), ray_dir, max_toi, true, filter)
        {
            spawn_tree(&mut commands, x, y - hit, &mut materials, &mut meshes);
        }
    }
    trees_state.set(TreesState::Spawned);
}

#[derive(Component)]
pub struct Tree;

pub const TREE_COLLISION_GROUP: Group = Group::GROUP_3;

fn spawn_tree(
    commands: &mut Commands,
    x: f32,
    y: f32,
    materials: &mut ResMut<Assets<ColorMaterial>>,
    meshes: &mut ResMut<Assets<Mesh>>,
) {
    let tree_size = Vec2::new(16., 180.);
    commands.spawn((
        Tree,
        Name::new("Tree"),
        MaterialMesh2dBundle {
            transform: Transform::from_xyz(x, y + tree_size.y / 2., 2.),
            material: materials.add(Color::rgb(0.29, 0.196, 0.101).into()),
            mesh: meshes.add(Mesh::from(shape::Quad::new(tree_size))).into(),
            ..default()
        },
        RigidBody::Fixed,
        Collider::cuboid(tree_size.x / 2., tree_size.y / 2.),
        CollisionGroups::new(
            TREE_COLLISION_GROUP,
            TERRAIN_COLLISION_GROUP | PICKER_COLLISION_GROUP,
        ),
    ));
}

pub struct TreeDamageEvent {
    pub tree: Entity,
    pub damage: u32,
}

fn update_tree_health(
    mut commands: Commands,
    mut tree_damage_events: EventReader<TreeDamageEvent>,
    mut tree_health_query: Query<&mut Health, With<Tree>>,
) {
    for tree_damage_event in tree_damage_events.iter() {
        if let Ok(mut tree_health) = tree_health_query.get_mut(tree_damage_event.tree) {
            tree_health.0 = tree_health.0.saturating_sub(tree_damage_event.damage);
        } else {
            if let Some(mut tree_entity_commands) = commands.get_entity(tree_damage_event.tree) {
                tree_entity_commands
                    .insert(Health(100u32.saturating_sub(tree_damage_event.damage)));
            }
        }
    }
}

pub struct TreeDestroyedEvent {
    pub tree: Entity,
}

#[derive(Component)]
pub struct Log;

pub const OBJECT_COLLISION_GROUP: Group = Group::GROUP_5;

fn destroy_trees(
    mut commands: Commands,
    tree_health_query: Query<(&Health, &GlobalTransform, Entity), With<Tree>>,
    mut tree_destroyed_events: EventWriter<TreeDestroyedEvent>,
    rapier_context: Res<RapierContext>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (tree_health, tree_transform, tree_entity) in &tree_health_query {
        if tree_health.0 == 0 {
            commands.entity(tree_entity).despawn_recursive();
            tree_destroyed_events.send(TreeDestroyedEvent { tree: tree_entity });

            let ray_origin = tree_transform.translation().truncate();
            let ray_dir = Vec2::new(0.0, -1.0);
            let max_toi = 200.;
            let solid = true;
            let filter: QueryFilter =
                CollisionGroups::new(PICKER_COLLISION_GROUP, TERRAIN_COLLISION_GROUP).into();

            if let Some(intersection) =
                rapier_context.cast_ray_and_get_normal(ray_origin, ray_dir, max_toi, solid, filter)
            {
                let log_size = Vec2::new(15., 10.);
                commands.spawn((
                    Log,
                    Name::new("Log"),
                    BuildingMaterial,
                    MaterialMesh2dBundle {
                        transform: Transform::from_xyz(
                            intersection.1.point.x,
                            intersection.1.point.y + log_size.y / 2.,
                            2.,
                        ),
                        material: materials.add(Color::rgb(0.29, 0.196, 0.101).into()),
                        mesh: meshes.add(Mesh::from(shape::Quad::new(log_size))).into(),
                        ..default()
                    },
                    RigidBody::Fixed,
                    Collider::cuboid(log_size.x / 2., log_size.y / 2.),
                    CollisionGroups::new(OBJECT_COLLISION_GROUP, TERRAIN_COLLISION_GROUP),
                ));
            }
        }
    }
}
