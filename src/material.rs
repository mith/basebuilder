use bevy::{prelude::*, reflect::TypeUuid};
use bevy_common_assets::ron::RonAssetPlugin;

pub struct MaterialPlugin;

impl Plugin for MaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(RonAssetPlugin::<Materials>::new(&["materials.ron"]))
            .add_asset::<Materials>()
            .add_state::<MaterialsState>()
            .add_system(load_materials.in_schedule(OnEnter(MaterialsState::Loading)))
            .add_system(setup_materials.run_if(in_state(MaterialsState::Loading)));
    }
}

#[derive(serde::Deserialize, Clone)]
pub struct Material {
    pub name: String,
    pub color: Color,
}

#[derive(serde::Deserialize, TypeUuid)]
#[uuid = "2a18e173-bc60-4aa2-a02b-a63d97622ab0"]
struct Materials(Vec<Material>);

#[derive(Resource)]
struct MaterialsHandle(Handle<Materials>);

fn load_materials(mut commands: Commands, asset_server: Res<AssetServer>) {
    let materials = asset_server.load("base.materials.ron");
    commands.insert_resource(MaterialsHandle(materials));
}

#[derive(Resource)]
pub struct MaterialProperties(pub Vec<Material>);

#[derive(States, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum MaterialsState {
    #[default]
    Loading,
    Loaded,
}

fn setup_materials(
    mut commands: Commands,
    materials: Res<MaterialsHandle>,
    materials_assets: Res<Assets<Materials>>,
    mut state: ResMut<NextState<MaterialsState>>,
) {
    if let Some(materials) = materials_assets.get(&materials.0) {
        commands.insert_resource(MaterialProperties(materials.0.clone()));
        info!("Materials loaded");
        state.set(MaterialsState::Loaded);
    }
}
