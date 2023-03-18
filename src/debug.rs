use bevy::{input::common_conditions::input_toggle_active, prelude::*};
#[cfg(feature = "inspector")]
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rapier2d::render::{DebugRenderContext, RapierDebugRenderPlugin};

fn toggle_physics_debug(
    mut debug_render: ResMut<DebugRenderContext>,
    keyboard_input: Res<Input<KeyCode>>,
) {
    debug_render.enabled = !debug_render.enabled;
}

pub(crate) struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "inspector")]
        app.add_plugin(
            WorldInspectorPlugin::default().run_if(input_toggle_active(false, KeyCode::Escape)),
        );

        app.add_plugin(RapierDebugRenderPlugin {
            enabled: false,
            ..default()
        })
        .add_system(toggle_physics_debug.run_if(input_toggle_active(false, KeyCode::Escape)));
    }
}
