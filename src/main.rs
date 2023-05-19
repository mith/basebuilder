use ai_controller::AiControllerPlugin;
use bevy::prelude::*;

use app_state::AppStatePlugin;
use bevy_ecs_tilemap::TilemapPlugin;
use bevy_proto::prelude::ProtoPlugin;
use bevy_rapier2d::prelude::{NoUserData, RapierPhysicsPlugin};
use creep::CreepPlugin;
use cursor_position::CursorPositionPlugin;
use debug::DebugPlugin;
use dwarf::DwarfPlugin;
use gravity::GravityPlugin;
use health::HealthPlugin;
use hit::HitPlugin;
use hovered_tile::HoveredTilePlugin;
use item::ItemPlugin;
use load::LoadPlugin;
use main_camera::MainCameraPlugin;
use material::MaterialPlugin;
use movement::MovementPlugin;
use pan_zoom_camera2d::PanZoomCamera2dPlugin;
use structure::StructurePlugin;
use terrain::TerrainPlugin;
use terrain_settings::TerrainSettingsPlugin;

mod ai_controller;
mod app_state;
mod crafting;
mod creep;
mod cursor_position;
mod debug;
mod dwarf;
mod gravity;
mod health;
mod hit;
mod hovered_tile;
mod inventory;
mod item;
mod load;
mod main_camera;
mod material;
mod movement;
mod pan_zoom_camera2d;
mod structure;
mod terrain;
mod terrain_settings;

fn main() {
    let mut app = App::new();

    app.insert_resource(Msaa::Sample8)
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        fit_canvas_to_parent: true,
                        ..default()
                    }),
                    ..default()
                })
                // .set(AssetPlugin {
                //     watch_for_changes: true,
                //     ..default()
                // })
                .set(ImagePlugin::default_nearest()),
        )
        .insert_resource(ClearColor(Color::BLACK));
    // Add third-party plugins
    app.add_plugin(TilemapPlugin)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.))
        .add_plugin(ProtoPlugin::new());

    // Add crate plugins
    app.add_plugin(AppStatePlugin)
        .add_plugin(LoadPlugin)
        .add_plugin(DebugPlugin)
        .add_plugin(CursorPositionPlugin)
        .add_plugin(PanZoomCamera2dPlugin)
        .add_plugin(MaterialPlugin)
        .add_plugin(ItemPlugin)
        .add_plugin(TerrainSettingsPlugin)
        .add_plugin(TerrainPlugin)
        .add_plugin(CreepPlugin)
        .add_plugin(HoveredTilePlugin)
        .add_plugin(AiControllerPlugin)
        .add_plugin(GravityPlugin)
        .add_plugin(HealthPlugin)
        .add_plugin(MovementPlugin)
        .add_plugin(HitPlugin)
        .add_plugin(StructurePlugin)
        .add_plugin(MainCameraPlugin)
        .add_plugin(DwarfPlugin);

    app.run();
}
