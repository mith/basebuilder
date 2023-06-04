use bevy::{
    ecs::system::SystemParam,
    prelude::*,
    utils::{HashMap, HashSet},
};

pub struct BuildingMaterialPlugin;

impl Plugin for BuildingMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BuildingMaterialRegistry>().add_systems(
            (register_building_material, deregister_building_material).in_set(BuildingMaterialSet),
        );
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct BuildingMaterialSet;

#[derive(Component)]
pub struct BuildingMaterial;

#[derive(Resource, Default)]
struct BuildingMaterialRegistry {
    resources: HashMap<Name, HashSet<Entity>>,
}

impl BuildingMaterialRegistry {
    fn get_all(&self, name: &Name) -> Option<&HashSet<Entity>> {
        self.resources.get(name)
    }
}

fn register_building_material(
    mut commands: Commands,
    mut building_material_registry: ResMut<BuildingMaterialRegistry>,
    building_material_query: Query<(Entity, &Name), Added<BuildingMaterial>>,
) {
    for (entity, name) in building_material_query.iter() {
        building_material_registry
            .resources
            .entry(name.clone())
            .or_insert_with(HashSet::new)
            .insert(entity);
    }
}

fn deregister_building_material(
    mut commands: Commands,
    mut resource_registry: ResMut<BuildingMaterialRegistry>,
    mut removed_resources: RemovedComponents<BuildingMaterial>,
) {
    for entity in removed_resources.iter() {
        resource_registry.resources.retain(|_, entities| {
            entities.remove(&entity);
            !entities.is_empty()
        });
    }
}

#[derive(SystemParam)]
pub struct BuildingMaterialLocator<'w, 's> {
    building_material_registry: ResMut<'w, BuildingMaterialRegistry>,
    query: Query<'w, 's, &'static GlobalTransform, With<BuildingMaterial>>,
}

impl BuildingMaterialLocator<'_, '_> {
    pub fn get_closest(&self, name: &Name, pos: Vec3) -> Option<Entity> {
        self.building_material_registry
            .get_all(name)
            .and_then(|entities| {
                entities
                    .iter()
                    .filter_map(|entity| {
                        self.query
                            .get(*entity)
                            .map(|transform| {
                                (entity, transform.translation().distance_squared(pos))
                            })
                            .ok()
                    })
                    .min_by(|(_, dist1), (_, dist2)| dist1.partial_cmp(dist2).unwrap())
                    .map(|(entity, _)| *entity)
            })
    }
}
