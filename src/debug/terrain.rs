use bevy::prelude::{Color, Gizmos, Plugin, Update, Vec2};

pub struct TerrainDebugPlugin;

impl Plugin for TerrainDebugPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(Update, render_terrain_gizmos);
    }
}

fn render_terrain_gizmos(mut gizmos: Gizmos) {
    gizmos.circle_2d(Vec2::ZERO, 4., Color::RED);
}
