use bevy::prelude::*;

use crate::{app_state::AppState, pan_zoom_camera2d::PanZoomCamera2dBundle};

pub struct MainCameraPlugin;

impl Plugin for MainCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(spawn_camera.in_schedule(OnEnter(AppState::Game)));
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(PanZoomCamera2dBundle::default());
}
