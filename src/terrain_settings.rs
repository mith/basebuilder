use bevy::{
    prelude::*,
    reflect::{TypePath, TypeUuid},
    utils::HashMap,
};
use bevy_common_assets::ron::RonAssetPlugin;
use terrain_gen::TerrainGeneratorSettings;

use crate::material::MaterialProperties;

pub struct TerrainSettingsPlugin;

impl Plugin for TerrainSettingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RonAssetPlugin::<TerrainSettingsRaw>::new(&[
            "terrain_settings.ron",
        ]))
        .register_type::<TerrainSettings>()
        .add_asset::<TerrainSettingsRaw>()
        .add_state::<TerrainSettingsState>()
        .add_systems(
            OnEnter(TerrainSettingsState::Loading),
            load_terrain_settings,
        )
        .add_systems(
            Update,
            setup_terrain_settings
                .run_if(in_state(TerrainSettingsState::Loading))
                .run_if(resource_exists::<MaterialProperties>()),
        );
    }
}

#[derive(serde::Deserialize, Clone, TypeUuid, TypePath)]
#[uuid = "66ab7e1f-9767-4d9a-a3eb-db238bc75603"]
struct TerrainSettingsRaw {
    width: u32,
    height: u32,
    cell_size: f32,
    ore_incidences: HashMap<String, f32>,
    seed: u32,
}

#[derive(Resource)]
struct TerrainSettingsHandle(Handle<TerrainSettingsRaw>);

fn load_terrain_settings(mut commands: Commands, asset_server: Res<AssetServer>) {
    let terrain_settings = asset_server.load("base.terrain_settings.ron");
    commands.insert_resource(TerrainSettingsHandle(terrain_settings));
}

#[derive(Resource, Clone, Debug, Reflect)]
pub struct TerrainSettings {
    pub width: u32,
    pub height: u32,
    pub cell_size: f32,
    pub ore_incidences: HashMap<u16, f32>,
    pub seed: u32,
}

impl From<TerrainSettings> for TerrainGeneratorSettings {
    fn from(settings: TerrainSettings) -> Self {
        Self {
            width: settings.width,
            height: settings.height,
            cell_size: settings.cell_size,
            ore_incidences: settings.ore_incidences,
            seed: settings.seed,
        }
    }
}

#[derive(States, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum TerrainSettingsState {
    #[default]
    Loading,
    Loaded,
}

fn setup_terrain_settings(
    mut commands: Commands,
    material_properties: Res<MaterialProperties>,
    terrain_settings: Res<TerrainSettingsHandle>,
    terrain_settings_assets: Res<Assets<TerrainSettingsRaw>>,
    mut state: ResMut<NextState<TerrainSettingsState>>,
) {
    if let Some(terrain_settings) = terrain_settings_assets.get(&terrain_settings.0) {
        let mut ore_incidences = HashMap::new();
        for (ore, incidence) in &terrain_settings.ore_incidences {
            // find the material id from the name
            let material_id = material_properties
                .0
                .iter()
                .position(|material| material.name == *ore)
                .expect("Ore not found in material properties");
            ore_incidences.insert(material_id as u16, *incidence);
        }
        commands.insert_resource(TerrainSettings {
            width: terrain_settings.width,
            height: terrain_settings.height,
            cell_size: terrain_settings.cell_size,
            ore_incidences,
            seed: terrain_settings.seed,
        });
        info!("Terrain settings loaded");
        state.set(TerrainSettingsState::Loaded);
    }
}
