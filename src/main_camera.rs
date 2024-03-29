use bevy::prelude::*;

use crate::{main_state::MainState, pan_zoom_camera2d::PanZoomCamera2dBundle};

pub struct MainCameraPlugin;

impl Plugin for MainCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(MainState::Game), spawn_camera);
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(PanZoomCamera2dBundle {
        camera: Camera2dBundle { ..default() },
        ..default()
    });
}
