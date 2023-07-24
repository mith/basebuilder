use actions::ActionsPlugin;
use bevy::prelude::*;

use bevy_ecs_tilemap::TilemapPlugin;
use bevy_egui::EguiPlugin;
use bevy_rapier2d::prelude::{NoUserData, RapierPhysicsPlugin};

use ai_controller::AiControllerPlugin;
use climbable::ClimbablePlugin;
use cursor_position::CursorPositionPlugin;
use debug::DebugPlugin;
use designation_layer::DesignationLayerPlugin;
use dwarf::DwarfPlugin;
use gravity::GravityPlugin;
use health::HealthPlugin;
use hit::HitPlugin;
use hovered_tile::HoveredTilePlugin;
use item::ItemPlugin;
use labor::LaborPlugin;
use ladder::LadderPlugin;
use load::LoadPlugin;
use main_camera::MainCameraPlugin;
use main_state::MainStatePlugin;
use material::MaterialPlugin;
use movement::MovementPlugin;
use pan_zoom_camera2d::PanZoomCamera2dPlugin;
use resource::BuildingMaterialPlugin;
use terrain::TerrainPlugin;
use terrain_settings::TerrainSettingsPlugin;
use toolbar::ToolbarPlugin;
use tree::TreePlugin;
use world_generation::WorldGenerationPlugin;

mod actions;
mod ai_controller;
mod climbable;
mod crafting;
mod cursor_position;
mod debug;
mod designation_layer;
mod dwarf;
mod gravity;
mod health;
mod hit;
mod hovered_tile;
mod inventory;
mod item;
mod labor;
mod ladder;
mod load;
mod main_camera;
mod main_state;
mod material;
mod movement;
mod pan_zoom_camera2d;
mod pathfinding;
mod resource;
mod terrain;
mod terrain_settings;
mod toolbar;
mod tree;
mod world_generation;

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
                .set(ImagePlugin::default_nearest()),
        )
        .insert_resource(ClearColor(Color::BLACK));
    // Add third-party plugins
    app.add_plugin(TilemapPlugin)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.))
        .add_plugin(EguiPlugin);

    // Add crate plugins
    app.add_plugin(MainStatePlugin)
        .add_plugin(LoadPlugin)
        .add_plugin(DebugPlugin)
        .add_plugin(CursorPositionPlugin)
        .add_plugin(PanZoomCamera2dPlugin)
        .add_plugin(MaterialPlugin)
        .add_plugin(ItemPlugin)
        .add_plugin(TerrainSettingsPlugin)
        .add_plugin(TerrainPlugin)
        .add_plugin(WorldGenerationPlugin)
        .add_plugin(HoveredTilePlugin)
        .add_plugin(AiControllerPlugin)
        .add_plugin(GravityPlugin)
        .add_plugin(HealthPlugin)
        .add_plugin(MovementPlugin)
        .add_plugin(ClimbablePlugin)
        .add_plugin(HitPlugin)
        .add_plugin(MainCameraPlugin)
        .add_plugin(DwarfPlugin)
        .add_plugin(LaborPlugin)
        .add_plugin(ActionsPlugin)
        .add_plugin(DesignationLayerPlugin)
        .add_plugin(ToolbarPlugin)
        .add_plugin(TreePlugin)
        .add_plugin(BuildingMaterialPlugin)
        .add_plugin(LadderPlugin);

    app.run();
}
