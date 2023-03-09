use bevy::prelude::*;
use bevy_ecs_tilemap::TilemapPlugin;
#[cfg(feature = "inspector")]
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rapier2d::{
    prelude::{NoUserData, RapierPhysicsPlugin},
    render::RapierDebugRenderPlugin,
};

mod basebuilder;
mod cursor_position;
mod pan_zoom_camera2d;
mod terrain;
mod dude;
mod hovered_tile;

fn main() {
    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                window: WindowDescriptor {
                    fit_canvas_to_parent: true,
                    #[cfg(not(feature = "inspector"))]
                    cursor_visible: false,
                    ..default()
                },
                ..default()
            })
            .set(ImagePlugin::default_nearest()),
    )
    .insert_resource(ClearColor(Color::BLACK));

    #[cfg(feature = "inspector")]
    app.add_plugin(WorldInspectorPlugin);

    // Add third-party plugins
    app.add_plugin(TilemapPlugin)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(16.))
        .add_plugin(RapierDebugRenderPlugin::default());

    // Add crate plugins
    app.add_plugin(cursor_position::CursorPositionPlugin)
        .add_plugin(pan_zoom_camera2d::PanZoomCamera2dPlugin)
        .add_plugin(terrain::TerrainPlugin)
        .add_plugin(basebuilder::BasebuilderPlugin)
        .add_plugin(dude::DudePlugin);

    app.run();
}
