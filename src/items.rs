
use bevy::{prelude::*, reflect::TypeUuid};
use bevy_common_assets::ron::RonAssetPlugin;

#[derive(serde::Deserialize, Clone)]
pub(crate) struct Item {
    pub(crate) name: String,
    pub(crate) color: Color,
}

#[derive(serde::Deserialize, TypeUuid)]
#[uuid = "2a18e173-bc60-4aa2-a02b-a63d97622ab0"]
struct Items(Vec<Item>);

#[derive(Resource)]
struct ItemsHandle(Handle<Items>);

fn load_items(mut commands: Commands, asset_server: Res<AssetServer>) {
    let items = asset_server.load("base.items.ron");
    commands.insert_resource(ItemsHandle(items));
}

#[derive(Resource)]
pub(crate) struct ItemProperties(pub Vec<Item>);

#[derive(States, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub(crate) enum ItemsState {
    #[default]
    Loading,
    Loaded,
}

fn setup_items(
    mut commands: Commands,
    items: Res<ItemsHandle>,
    items_assets: Res<Assets<Items>>,
    mut state: ResMut<NextState<ItemsState>>,
) {
    if let Some(items) = items_assets.get(&items.0) {
        commands.insert_resource(ItemProperties(items.0.clone()));
        state.set(ItemsState::Loaded);
    }
}

pub(crate) struct ItemsPlugin;

impl Plugin for ItemsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(RonAssetPlugin::<Items>::new(&["items.ron"]))
            .add_asset::<Items>()
            .add_state::<ItemsState>()
            .add_system(load_items.in_schedule(OnEnter(ItemsState::Loading)))
            .add_system(setup_items.run_if(in_state(ItemsState::Loading)));
    }
}
