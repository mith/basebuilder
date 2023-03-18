use ai_controller::AiControllerPlugin;
use bevy::prelude::*;

use app_state::AppStatePlugin;
use bevy_ecs_tilemap::TilemapPlugin;
use bevy_rapier2d::prelude::{NoUserData, RapierPhysicsPlugin};
use creep::CreepPlugin;
use cursor_position::CursorPositionPlugin;
use debug::DebugPlugin;
use gravity::GravityPlugin;
use gun::GunPlugin;
use health::HealthPlugin;
use hit::HitPlugin;
use hovered_tile::HoveredTilePlugin;
use load::LoadPlugin;
use material::MaterialPlugin;
use movement::MovementPlugin;
use pan_zoom_camera2d::PanZoomCamera2dPlugin;
use player::PlayerPlugin;
use player_controller::PlayerControllerPlugin;
use terrain::TerrainPlugin;

mod ai_controller;
mod app_state;
mod creep;
mod cursor_position;
mod debug;
mod gravity;
mod gun;
mod health;
mod hit;
mod hovered_tile;
mod load;
mod material;
mod movement;
mod pan_zoom_camera2d;
mod player;
mod player_controller;
mod terrain;

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
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.));

    // Add crate plugins
    app.add_plugin(AppStatePlugin)
        .add_plugin(LoadPlugin)
        .add_plugin(DebugPlugin)
        .add_plugin(CursorPositionPlugin)
        .add_plugin(PanZoomCamera2dPlugin)
        .add_plugin(MaterialPlugin)
        .add_plugin(TerrainPlugin)
        .add_plugin(CreepPlugin)
        .add_plugin(HoveredTilePlugin)
        .add_plugin(PlayerPlugin)
        .add_plugin(PlayerControllerPlugin)
        .add_plugin(AiControllerPlugin)
        .add_plugin(GravityPlugin)
        .add_plugin(HealthPlugin)
        .add_plugin(MovementPlugin)
        .add_plugin(GunPlugin)
        .add_plugin(HitPlugin);

    app.run();
}
