use bevy::prelude::*;

use crate::pan_zoom_camera2d::PanZoomCamera2dBundle;

#[derive(Resource)]
pub(crate) struct BasebuilderConfig {
}

impl Default for BasebuilderConfig {
    fn default() -> Self {
        Self {
        }
    }
}

pub(crate) fn setup_camera(mut commands: Commands, config: Res<BasebuilderConfig>) {
    commands.spawn(PanZoomCamera2dBundle::default());
}

pub(crate) struct BasebuilderPlugin;

impl Plugin for BasebuilderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BasebuilderConfig>()
            .add_startup_system(setup_camera);

    }
}