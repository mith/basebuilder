use actions::ActionsPlugin;
use bevy::prelude::*;

use bevy_ecs_tilemap::TilemapPlugin;
use bevy_egui::EguiPlugin;
use bevy_rapier2d::prelude::{NoUserData, RapierPhysicsPlugin};

use big_brain::BigBrainPlugin;
use building_material::BuildingMaterialPlugin;
use climbable::ClimbablePlugin;
use cursor_position::CursorPositionPlugin;
use debug::DebugPlugin;
use designation_layer::DesignationLayerPlugin;
use dwarf::DwarfPlugin;
use gravity::GravityPlugin;
use health::HealthPlugin;
use hit::HitPlugin;
use hovered_tile::HoveredTilePlugin;
use labor::LaborPlugin;
use ladder::LadderPlugin;
use load::LoadPlugin;
use main_camera::MainCameraPlugin;
use main_state::MainStatePlugin;
use material::MaterialPlugin;
use movement::MovementPlugin;
use pan_zoom_camera2d::PanZoomCamera2dPlugin;
use terrain::TerrainPlugin;
use terrain_settings::TerrainSettingsPlugin;
use toolbar::ToolbarPlugin;
use tree::TreePlugin;
use world_generation::WorldGenerationPlugin;

mod actions;
mod building_material;
mod climbable;
mod cursor_position;
mod debug;
mod designation_layer;
mod dwarf;
mod gravity;
mod health;
mod hit;
mod hovered_tile;
mod labor;
mod ladder;
mod load;
mod main_camera;
mod main_state;
mod material;
mod movement;
mod pan_zoom_camera2d;
mod pathfinding;
mod terrain;
mod terrain_settings;
mod toolbar;
mod tree;
mod util;
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
    app.add_plugins((
        TilemapPlugin,
        RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.),
        EguiPlugin,
        BigBrainPlugin::new(PreUpdate),
    ));

    // Add crate plugins
    app.add_plugins((
        MainStatePlugin,
        LoadPlugin,
        DebugPlugin,
        CursorPositionPlugin,
        PanZoomCamera2dPlugin,
        MaterialPlugin,
        TerrainSettingsPlugin,
        TerrainPlugin,
        WorldGenerationPlugin,
        HoveredTilePlugin,
    ));

    app.add_plugins((
        GravityPlugin,
        HealthPlugin,
        MovementPlugin,
        ClimbablePlugin,
        HitPlugin,
        MainCameraPlugin,
        DwarfPlugin,
        LaborPlugin,
        ActionsPlugin,
        DesignationLayerPlugin,
        ToolbarPlugin,
        TreePlugin,
        BuildingMaterialPlugin,
        LadderPlugin,
    ));

    app.run();
}
