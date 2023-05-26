use bevy::{prelude::*, reflect::TypeUuid};
use bevy_common_assets::ron::RonAssetPlugin;

pub struct ItemPlugin;

impl Plugin for ItemPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(RonAssetPlugin::<Items>::new(&["items.ron"]))
            .add_asset::<Items>()
            .add_state::<ItemsState>()
            .add_system(load_items.in_schedule(OnEnter(ItemsState::Loading)))
            .add_system(setup_items.run_if(in_state(ItemsState::Loading)));
    }
}

#[derive(serde::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct U8Color {
    r: u8,
    g: u8,
    b: u8,
}

#[derive(serde::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Item {
    pub name: String,
    pub color: U8Color,
}

impl Item {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            color: U8Color { r: 0, g: 0, b: 0 },
        }
    }

    pub fn color(&self) -> Color {
        Color::rgb_u8(self.color.r, self.color.g, self.color.b)
    }
}

#[derive(serde::Deserialize, TypeUuid)]
#[uuid = "b0e5fa81-d40d-4792-bc09-4f8778a649a3"]
struct Items(Vec<Item>);

#[derive(Resource)]
struct ItemsHandle(Handle<Items>);

fn load_items(mut commands: Commands, asset_server: Res<AssetServer>) {
    let items = asset_server.load("base.items.ron");
    commands.insert_resource(ItemsHandle(items));
}

#[derive(Resource)]
pub struct ItemProperties(pub Vec<Item>);

#[derive(States, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ItemsState {
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
