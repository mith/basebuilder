use app_state::AppState;
use app_state::AppStatePlugin;
use basebuilder::BasebuilderPlugin;
use bevy::prelude::*;
use bevy_ecs_tilemap::TilemapPlugin;
#[cfg(feature = "inspector")]
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rapier2d::{
    prelude::{NoUserData, RapierPhysicsPlugin},
    render::RapierDebugRenderPlugin,
};
use cursor_position::CursorPositionPlugin;
use dude::DudePlugin;
use hovered_tile::HoveredTilePlugin;
use load::LoadPlugin;
use material::{MaterialPlugin, MaterialsState};
use pan_zoom_camera2d::PanZoomCamera2dPlugin;
use terrain::TerrainPlugin;

mod app_state;
mod basebuilder;
mod cursor_position;
mod dude;
mod hovered_tile;
mod load;
mod material;
mod pan_zoom_camera2d;
mod terrain;

fn main() {
    let mut app = App::new();

    app.add_plugins(
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

    #[cfg(feature = "inspector")]
    app.add_plugin(WorldInspectorPlugin::default());

    // Add third-party plugins
    app.add_plugin(TilemapPlugin)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(16.))
        .add_plugin(RapierDebugRenderPlugin::default());

    // Add crate plugins
    app.add_plugin(AppStatePlugin)
        .add_plugin(LoadPlugin)
        .add_plugin(CursorPositionPlugin)
        .add_plugin(PanZoomCamera2dPlugin)
        .add_plugin(MaterialPlugin)
        .add_plugin(TerrainPlugin)
        .add_plugin(BasebuilderPlugin)
        .add_plugin(HoveredTilePlugin)
        .add_plugin(DudePlugin);

    app.run();
}
